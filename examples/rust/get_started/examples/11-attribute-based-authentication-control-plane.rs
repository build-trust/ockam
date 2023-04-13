use std::sync::Arc;
use std::time::Duration;

use hello_ockam::{create_token, import_project};
use ockam::abac::AbacAccessControl;
use ockam::identity::credential::OneTimeCode;
use ockam::identity::{
    AuthorityService, RemoteCredentialsRetriever, RemoteCredentialsRetrieverInfo, SecureChannelListenerOptions,
    SecureChannelOptions, TrustContext, TrustMultiIdentifiersPolicy,
};
use ockam::remote::RemoteForwarderOptions;
use ockam::{node, route, Context, Result, TcpOutletOptions};
use ockam_api::authenticator::direct::TokenAcceptorClient;
use ockam_api::{multiaddr_to_route, DefaultAddress};
use ockam_core::flow_control::FlowControls;
use ockam_node::RpcClient;
use ockam_transport_tcp::TcpTransportExtension;

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
    // Create a node with default implementations
    let node = node(ctx);
    // Initialize the TCP transport
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity for the control node
    let control_plane = node.create_identity().await?;

    // 2. create a secure channel to the authority
    //    to retrieve the node credential

    // Import the authority identity and route from the information file
    let project = import_project(project_information_path, node.identities()).await?;

    let tcp_route = multiaddr_to_route(&project.authority_route(), &tcp).await.unwrap(); // FIXME: Handle error
    let options = SecureChannelOptions::new()
        .with_trust_policy(TrustMultiIdentifiersPolicy::new(vec![project.authority_identifier()]));

    // create a secure channel to the authority
    // when creating the channel we check that the opposite side is indeed presenting the authority identity
    let secure_channel = node
        .create_secure_channel_extended(&control_plane, tcp_route.route, options, Duration::from_secs(120))
        .await?;

    let token_acceptor = TokenAcceptorClient::new(
        RpcClient::new(
            route![secure_channel.clone(), DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR],
            node.context(),
        )
        .await?,
    );
    token_acceptor.present_token(&token).await?;

    let tcp_project_session = multiaddr_to_route(&project.authority_route(), &tcp).await.unwrap(); // FIXME: Handle error

    // Create a trust context that will be used to authenticate credential exchanges
    let trust_context = TrustContext::new(
        "trust_context_id".to_string(),
        Some(AuthorityService::new(
            node.identities().identities_reader(),
            node.credentials(),
            project.authority_identifier(),
            Some(Arc::new(RemoteCredentialsRetriever::new(
                node.secure_channels(),
                RemoteCredentialsRetrieverInfo::new(
                    project.authority_identifier(),
                    tcp_project_session.route,
                    DefaultAddress::CREDENTIAL_ISSUER.into(),
                ),
            ))),
        )),
    );

    let credential = trust_context
        .authority()?
        .credential(node.context(), &control_plane)
        .await?;

    println!("{credential}");

    // start a credential exchange worker which will be
    // later on to exchange credentials with the edge node
    node.credentials_server()
        .start(
            node.context(),
            trust_context,
            project.authority_identifier(),
            "credential_exchange".into(),
            true,
        )
        .await?;

    // 3. create an access control policy checking the value of the "component" attribute of the caller
    let access_control = AbacAccessControl::create(node.repository(), "component", "edge");

    // 4. create a tcp outlet with the above policy
    tcp.create_outlet(
        "outlet",
        "127.0.0.1:5000",
        TcpOutletOptions::new().with_incoming_access_control_impl(access_control),
    )
    .await?;

    // 5. create a forwarder on the Ockam orchestrator

    let tcp_project_route = multiaddr_to_route(&project.route(), &tcp).await.unwrap(); // FIXME: Handle error
    let project_options =
        SecureChannelOptions::new().with_trust_policy(TrustMultiIdentifiersPolicy::new(vec![project.identifier()]));

    // create a secure channel to the project first
    // we make sure that we indeed connect to the correct project on the Orchestrator
    let secure_channel_address = node
        .create_secure_channel_extended(
            &control_plane,
            tcp_project_route.route,
            project_options,
            Duration::from_secs(120),
        )
        .await?;
    println!("secure channel to project: {secure_channel_address:?}");

    // present this node credential to the project
    node.credentials_server()
        .present_credential(
            node.context(),
            route![secure_channel_address.clone(), DefaultAddress::CREDENTIALS_SERVICE],
            credential,
        )
        .await?;

    // finally create a forwarder using the secure channel to the project
    let forwarder = node
        .create_static_forwarder(secure_channel_address, "control_plane1", RemoteForwarderOptions::new())
        .await?;
    println!("forwarder is {forwarder:?}");

    // 6. create a secure channel listener which will allow the edge node to
    //    start a secure channel when it is ready
    let sc_flow_control_id = FlowControls::generate_id();
    node.create_secure_channel_listener(
        &control_plane,
        "untrusted",
        SecureChannelListenerOptions::new(&sc_flow_control_id),
    )
    .await?;
    println!("created a secure channel listener");

    // don't stop the node
    Ok(())
}
