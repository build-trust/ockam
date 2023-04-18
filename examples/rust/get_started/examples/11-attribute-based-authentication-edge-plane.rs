use ockam::{node, MessageSendReceiveOptions, TcpInletOptions};
use ockam_core::compat::sync::Arc;
use std::time::Duration;

use hello_ockam::{create_token, import_project};
use ockam::abac::AbacAccessControl;
use ockam::identity::credential::OneTimeCode;
use ockam::identity::{
    identities, AuthorityService, RemoteCredentialsRetriever, RemoteCredentialsRetrieverInfo, SecureChannelOptions,
    TrustContext, TrustMultiIdentifiersPolicy,
};
use ockam::{route, Context, Result};
use ockam_api::authenticator::direct::TokenAcceptorClient;
use ockam_api::{multiaddr_to_route, DefaultAddress};
use ockam_core::flow_control::FlowControls;
use ockam_node::RpcClient;
use ockam_transport_tcp::TcpTransportExtension;

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
    // Create a node with default implementations
    let node = node(ctx);
    // Use the TCP transport
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity for the edge plane
    let edge_plane = node.create_identity().await?;

    // 2. create a secure channel to the authority
    //    to retrieve the node credential

    // Import the authority identity and route from the information file
    let project = import_project(project_information_path, node.identities_vault()).await?;

    let flow_controls = FlowControls::default();
    let tcp_authority_route = multiaddr_to_route(&project.authority_route(), &tcp, &flow_controls)
        .await
        .unwrap(); // FIXME: Handle error
    let authority_options = SecureChannelOptions::new()
        .with_trust_policy(TrustMultiIdentifiersPolicy::new(vec![project.authority_identifier()]));
    let options = if let Some(_session_id) = tcp_authority_route.flow_control_id {
        authority_options.as_consumer(&flow_controls)
    } else {
        authority_options
    };

    // create a secure channel to the authority
    // when creating the channel we check that the opposite side is indeed presenting the authority identity
    let secure_channel = node
        .create_secure_channel_extended(
            &edge_plane,
            tcp_authority_route.route,
            options,
            Duration::from_secs(120),
        )
        .await?;

    let token_acceptor = TokenAcceptorClient::new(
        RpcClient::new(
            route![secure_channel.clone(), DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR],
            &node.get_context().await?,
        )
        .await?,
    );
    token_acceptor.present_token(&token).await?;

    // Create a trust context that will be used to authenticate credential exchanges
    let tcp_project_session = multiaddr_to_route(&project.route(), &tcp, &flow_controls)
        .await
        .unwrap(); // FIXME: Handle error

    let trust_context = TrustContext::new(
        "trust_context_id".to_string(),
        Some(AuthorityService::new(
            node.credentials(),
            project.authority_identity(),
            Some(Arc::new(RemoteCredentialsRetriever::new(
                node.secure_channels(),
                RemoteCredentialsRetrieverInfo::new(
                    project.authority_identity(),
                    tcp_project_session.route,
                    DefaultAddress::CREDENTIAL_ISSUER.into(),
                ),
                flow_controls.clone(),
            ))),
        )),
    );

    let credential = trust_context
        .authority()?
        .credential(&node.get_context().await?, &edge_plane)
        .await?;

    println!("{credential}");

    // start a credential exchange worker which will be
    // later on to exchange credentials with the control node
    node.credentials_server()
        .start(
            &node.get_context().await?,
            trust_context,
            project.authority_identity(),
            "credential_exchange".into(),
            true,
        )
        .await?;

    // 3. create an access control policy checking the value of the "component" attribute of the caller
    let access_control = AbacAccessControl::create(identities().repository(), "component", "control");

    // 4. create a tcp inlet with the above policy

    let tcp_project_route = multiaddr_to_route(&project.route(), &tcp, &flow_controls)
        .await
        .unwrap(); // FIXME: Handle error
    let project_options =
        SecureChannelOptions::new().with_trust_policy(TrustMultiIdentifiersPolicy::new(vec![project.identifier()]));
    let project_options = if let Some(_flow_control_id) = tcp_project_route.flow_control_id {
        project_options.as_consumer(&flow_controls)
    } else {
        project_options
    };

    // 4.1 first created a secure channel to the project
    let secure_channel_address = node
        .create_secure_channel_extended(
            &edge_plane,
            tcp_project_route.route,
            project_options,
            Duration::from_secs(120),
        )
        .await?;
    println!("secure channel address to the project: {secure_channel_address:?}");

    // 4.2 and send this node credential to the project
    node.credentials_server()
        .present_credential(
            &node.get_context().await?,
            route![secure_channel_address.clone(), DefaultAddress::CREDENTIALS_SERVICE],
            credential.clone(),
            MessageSendReceiveOptions::new().with_flow_control(&flow_controls),
        )
        .await?;

    // 4.3 then create a secure channel to the control node (via its forwarder)
    let secure_channel_listener_route = route![secure_channel_address, "forward_to_control_plane1", "untrusted"];
    let secure_channel_to_control = node
        .create_secure_channel_extended(
            &edge_plane,
            secure_channel_listener_route.clone(),
            SecureChannelOptions::new(),
            Duration::from_secs(120),
        )
        .await?;

    println!("secure channel address to the control node: {secure_channel_to_control:?}");

    // 4.4 exchange credential with the control node
    node.credentials_server()
        .present_credential_mutual(
            &node.get_context().await?,
            route![secure_channel_to_control.clone(), "credential_exchange"],
            &[project.authority_identity()],
            credential,
            MessageSendReceiveOptions::new().with_flow_control(&flow_controls),
        )
        .await?;
    println!("credential exchange done");

    // 4.5 create a TCP inlet connected to the TCP outlet on the control node
    let outlet_route = route![secure_channel_to_control, "outlet"];
    let inlet = tcp
        .create_inlet(
            "127.0.0.1:7000",
            outlet_route.clone(),
            TcpInletOptions::new().with_incoming_access_control_impl(access_control),
        )
        .await?;
    println!("the inlet is {inlet:?}");

    // don't stop the node
    Ok(())
}
