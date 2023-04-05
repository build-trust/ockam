use ockam::AsyncTryClone;
use ockam_core::compat::sync::Arc;
use std::time::Duration;

use hello_ockam::{create_token, import_project};
use ockam::abac::AbacAccessControl;
use ockam::identity::authenticated_storage::AuthenticatedAttributeStorage;
use ockam::identity::credential::OneTimeCode;
use ockam::identity::{
    AuthorityInfo, Identity, SecureChannelTrustOptions, TrustContext, TrustEveryonePolicy, TrustMultiIdentifiersPolicy,
};
use ockam::{route, vault::Vault, Context, MessageSendReceiveOptions, Result, TcpInletTrustOptions, TcpTransport};
use ockam_api::authenticator::direct::{CredentialIssuerClient, RpcClient, TokenAcceptorClient};
use ockam_api::{create_tcp_session, CredentialIssuerInfo, CredentialIssuerRetriever, DefaultAddress};
use ockam_core::sessions::Sessions;

/// This node supports an "edge" server which can connect to a "control" node
/// in order to connect its TCP inlet to the "control" node TCP outlet
///
/// The connections go through the Ockam Orchestrator, via the control node Forwarder.
///
/// This example shows how to:
///
///   - retrieve credentials from an authority
///   - create a TCP inlet with some access control checking the authenticated attributes of the caller
///   - connect the TCP inlet to a server outlet
///
/// The node needs to be started with:
///
///  - a project.json file created with `ockam project information --output json  > project.json`
///  - a token created by an enroller node with `ockam project enroll --attribute component=edge`
///
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // create a token (with `ockam project enroll --attribute component=edge`)
    // In principle this token is provided by another node which has enrolling privileges for the
    // current project
    let token: OneTimeCode = create_token("component", "edge").await?;
    println!("token: {token:?}");

    // set the path of the project information file
    // In principle this file is provided by the enrolling node by running the command
    // `ockam project information --output json  > project.json`
    let project_information_path = "project.json";
    start_node(ctx, project_information_path, token).await
}

/// start the edge node
async fn start_node(ctx: Context, project_information_path: &str, token: OneTimeCode) -> Result<()> {
    // Use the TCP transport
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an Identity for the edge plane
    let vault = Vault::create();
    let edge_plane = Identity::create(&ctx, vault.clone()).await?;

    // 2. create a secure channel to the authority
    //    to retrieve the node credential

    // Import the authority identity and route from the information file
    let project = import_project(project_information_path, vault).await?;

    let sessions = Sessions::default();
    let tcp_authority_session = create_tcp_session(&project.authority_route(), &tcp, &sessions)
        .await
        .unwrap(); // FIXME: Handle error
    let authority_trust_options =
        SecureChannelTrustOptions::insecure_test().with_trust_policy(TrustMultiIdentifiersPolicy::new(vec![
            project.authority_public_identifier()
        ]));
    let trust_options = if let Some(session_id) = tcp_authority_session.session_id {
        authority_trust_options.as_consumer(&sessions, &session_id)
    } else {
        authority_trust_options
    };

    // create a secure channel to the authority
    // when creating the channel we check that the opposite side is indeed presenting the authority identity
    let secure_channel = edge_plane
        .create_secure_channel_extended(tcp_authority_session.route, trust_options, Duration::from_secs(120))
        .await?;

    let token_acceptor = TokenAcceptorClient::new(
        RpcClient::new(
            route![secure_channel.clone(), DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR],
            &ctx,
        )
        .await?,
    );
    token_acceptor.present_token(&token).await?;

    // Create a trust context that will be used to authenticate credential exchanges
    let trust_context = TrustContext::new(
        "trust_context_id".to_string(),
        Some(AuthorityInfo::new(
            project.authority_public_identity(),
            Some(Arc::new(CredentialIssuerRetriever::new(
                CredentialIssuerInfo::new(
                    hex::encode(project.authority_public_identity().export()?),
                    project.authority_route(),
                ),
                tcp.async_try_clone().await?,
            ))),
        )),
    );

    let credential = trust_context.authority()?.credential(&edge_plane).await?;

    println!("{credential}");

    // start a credential exchange worker which will be
    // later on to exchange credentials with the control node
    let storage = AuthenticatedAttributeStorage::new(edge_plane.authenticated_storage().clone());
    edge_plane
        .start_credential_exchange_worker(trust_context, "credential_exchange", true, Arc::new(storage))
        .await?;

    // 3. create an access control policy checking the value of the "component" attribute of the caller
    let access_control = AbacAccessControl::create(edge_plane.authenticated_storage().clone(), "component", "control");

    // 4. create a tcp inlet with the above policy

    let tcp_project_session = create_tcp_session(&project.route(), &tcp, &sessions).await.unwrap(); // FIXME: Handle error
    let project_trust_options = SecureChannelTrustOptions::insecure_test()
        .with_trust_policy(TrustMultiIdentifiersPolicy::new(vec![project.identifier()]));
    let project_trust_options = if let Some(session_id) = tcp_project_session.session_id {
        project_trust_options.as_consumer(&sessions, &session_id)
    } else {
        project_trust_options
    };

    // 4.1 first created a secure channel to the project
    let secure_channel_address = edge_plane
        .create_secure_channel_extended(
            tcp_project_session.route,
            project_trust_options,
            Duration::from_secs(120),
        )
        .await?;
    println!("secure channel address to the project: {secure_channel_address:?}");

    // 4.2 and send this node credential to the project
    edge_plane
        .present_credential(
            route![secure_channel_address.clone(), DefaultAddress::CREDENTIALS_SERVICE],
            &credential,
            MessageSendReceiveOptions::new().with_session(&sessions),
        )
        .await?;

    // 4.3 then create a secure channel to the control node (via its forwarder)
    let secure_channel_listener_route = route![secure_channel_address, "forward_to_control_plane1", "untrusted"];
    let secure_channel_to_control = edge_plane
        .create_secure_channel_extended(
            secure_channel_listener_route.clone(),
            SecureChannelTrustOptions::insecure_test(),
            Duration::from_secs(120),
        )
        .await?;

    println!("secure channel address to the control node: {secure_channel_to_control:?}");

    // 4.4 exchange credential with the control node
    let storage = AuthenticatedAttributeStorage::new(edge_plane.authenticated_storage().clone());
    edge_plane
        .present_credential_mutual(
            route![secure_channel_to_control.clone(), "credential_exchange"],
            vec![&project.authority_public_identity()],
            Arc::new(storage),
            &credential,
            MessageSendReceiveOptions::new().with_session(&sessions),
        )
        .await?;
    println!("credential exchange done");

    // 4.5 create a TCP inlet connected to the TCP outlet on the control node
    let outlet_route = route![secure_channel_to_control, "outlet"];
    let inlet = tcp
        .create_inlet(
            "127.0.0.1:7000",
            outlet_route.clone(),
            TcpInletTrustOptions::new().with_incoming_access_control_impl(access_control),
        )
        .await?;
    println!("the inlet is {inlet:?}");

    // don't stop the node
    Ok(())
}
