use hello_ockam::{create_token, import_project};
use ockam::identity::authenticated_storage::AuthenticatedAttributeStorage;
use ockam::identity::credential::{CredentialExchangeMode, OneTimeCode};
use ockam::identity::{
    AuthorityInfo, Identity, SecureChannelListenerOptions, SecureChannelOptions, TrustContext,
    TrustMultiIdentifiersPolicy,
};
use ockam::AsyncTryClone;

use ockam::abac::AbacAccessControl;
use ockam::remote::{RemoteForwarder, RemoteForwarderOptions};
use ockam::{route, vault::Vault, Context, MessageSendReceiveOptions, Result, TcpOutletOptions, TcpTransport};
use ockam_api::authenticator::direct::{RpcClient, TokenAcceptorClient};
use ockam_api::{multiaddr_to_route, CredentialIssuerInfo, CredentialIssuerRetriever, DefaultAddress};
use ockam_core::flow_control::FlowControls;
use std::sync::Arc;
use std::time::Duration;

/// This node supports a "control" server on which several "edge" devices can connect
///
/// The connections go through the Ockam Orchestrator, via a Forwarder, and a secure channel
/// can be established to forward messages to an outlet going to a local Python webserver.
///
///
/// This example shows how to:
///
///   - retrieve credentials from an authority
///   - create a Forwarder on the Ockam Orchestrator
///   - create a TCP outlet with some access control checking the authenticated attributes of the caller
///
/// The node needs to be started with:
///
///  - a project.json file created with `ockam project information --output json  > project.json`
///  - a token created by an enroller node with `ockam project enroll --attribute component=control`
///
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // create a token (with `ockam project enroll --attribute component=control`)
    // In principle this token is provided by another node which has enrolling privileges for the
    // current project
    let token: OneTimeCode = create_token("component", "control").await?;
    println!("token: {token:?}");

    // set the path of the project information file
    // In principle this file is provided by the enrolling node by running the command
    // `ockam project information --output json  > project.json`
    let project_information_path = "project.json";
    start_node(ctx, project_information_path, token).await
}

/// start the control node
async fn start_node(ctx: Context, project_information_path: &str, token: OneTimeCode) -> Result<()> {
    // Initialize the TCP transport
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an Identity for the control node
    let vault = Vault::create();
    let control_plane = Identity::create(&ctx, vault.clone()).await?;

    // 2. create a secure channel to the authority
    //    to retrieve the node credential

    // Import the authority identity and route from the information file
    let project = import_project(project_information_path, vault).await?;

    let flow_controls = FlowControls::default();
    let tcp_route = multiaddr_to_route(&project.authority_route(), &tcp, &flow_controls)
        .await
        .unwrap(); // FIXME: Handle error
    let options = SecureChannelOptions::new().with_trust_policy(TrustMultiIdentifiersPolicy::new(vec![
        project.authority_public_identifier()
    ]));
    let options = if let Some(_flow_control_id) = tcp_route.flow_control_id {
        options.as_consumer(&flow_controls).with_trust_policy(project)
    } else {
        options.with_trust_policy(project)
    };

    // create a secure channel to the authority
    // when creating the channel we check that the opposite side is indeed presenting the authority identity
    let secure_channel = control_plane
        .create_secure_channel_extended(tcp_route.route, options, Duration::from_secs(120))
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

    let credential = trust_context.authority()?.credential(&control_plane).await?;

    println!("{credential}");

    // 3. create an access control policy checking the value of the "component" attribute of the caller
    let access_control = AbacAccessControl::create(control_plane.authenticated_storage().clone(), "component", "edge");

    // 4. create a tcp outlet with the above policy
    tcp.create_outlet(
        "outlet",
        "127.0.0.1:5000",
        TcpOutletOptions::new().with_incoming_access_control_impl(access_control),
    )
    .await?;

    // 5. create a forwarder on the Ockam orchestrator

    let tcp_project_route = multiaddr_to_route(&project.route(), &tcp, &flow_controls)
        .await
        .unwrap(); // FIXME: Handle error
    let project_options =
        SecureChannelOptions::new().with_trust_policy(TrustMultiIdentifiersPolicy::new(vec![project.identifier()]));
    let project_options = if let Some(_flow_control_id) = tcp_project_route.flow_control_id {
        project_options
            .as_consumer(&flow_controls)
            .with_credential(credential)
            .with_credential_exchange_mode(CredentialExchangeMode::Oneway);
    } else {
        project_options
            .with_credential(credential)
            .with_credential_exchange_mode(CredentialExchangeMode::Oneway);
    };

    // create a secure channel to the project first
    // we make sure that we indeed connect to the correct project on the Orchestrator
    let secure_channel_address = control_plane
        .create_secure_channel_extended(tcp_project_route.route, project_options, Duration::from_secs(120))
        .await?;
    println!("secure channel to project: {secure_channel_address:?}");

    // finally create a forwarder using the secure channel to the project
    let forwarder = RemoteForwarder::create_static(
        &ctx,
        secure_channel_address,
        "control_plane1",
        RemoteForwarderOptions::new(),
    )
    .await?;
    println!("forwarder is {forwarder:?}");

    // 6. create a secure channel listener which will allow the edge node to
    //    start a secure channel when it is ready
    control_plane
        .create_secure_channel_listener("untrusted", SecureChannelListenerOptions::new())
        .await?;
    println!("created a secure channel listener");

    // don't stop the node
    Ok(())
}
