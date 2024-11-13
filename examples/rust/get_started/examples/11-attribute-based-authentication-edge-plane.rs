use hello_ockam::{create_token, import_project};
use ockam::abac::{IncomingAbac, OutgoingAbac};
use ockam::identity::{
    identities, RemoteCredentialRetrieverCreator, RemoteCredentialRetrieverInfo, SecureChannelOptions,
    TrustMultiIdentifiersPolicy,
};
use ockam::node;
use ockam::{route, Context, Result};
use ockam_api::authenticator::enrollment_tokens::TokenAcceptor;
use ockam_api::authenticator::one_time_code::OneTimeCode;
use ockam_api::nodes::NodeManager;
use ockam_api::{RemoteMultiaddrResolver, TransportRouteResolver};
use ockam_core::compat::sync::Arc;
use ockam_core::TryClone;
use ockam_multiaddr::MultiAddr;
use ockam_transport_tcp::{TcpInletOptions, TcpTransportExtension};

/// This node supports an "edge" server which can connect to a "control" node
/// in order to connect its TCP inlet to the "control" node TCP outlet
///
/// The connections go through the Ockam Orchestrator, via the control node Relay.
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
///  - a token created by an enroller node with `ockam project ticket --attribute component=edge`
///
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // create a token (with `ockam project ticket --attribute component=edge`)
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
    let node = node(ctx).await?;
    // Use the TCP transport
    let tcp = node.create_tcp_transport()?;

    // Create an Identity for the edge plane
    let edge_plane = node.create_identity().await?;

    // 2. create a secure channel to the authority node to
    //    to retrieve the node credential
    // create a secure channel to the authority
    // when creating the channel we check that the opposite side is indeed presenting the authority identity
    let authority_node = NodeManager::authority_node_client(
        &tcp,
        node.secure_channels().clone(),
        &edge_plane,
        &MultiAddr::try_from("/dnsaddr/localhost/tcp/5000")?,
        &node.create_identity().await?,
        None,
    )
    .await?;
    authority_node.present_token(node.context(), token).await.unwrap();

    let project = import_project(project_information_path, node.identities()).await?;

    let project_authority_route = TransportRouteResolver::default()
        .allow_tcp()
        .resolve(&project.route())?;

    // Create a credential retriever that will be used to obtain credentials
    let credential_retriever = Arc::new(RemoteCredentialRetrieverCreator::new(
        node.context().try_clone()?,
        Arc::new(tcp.clone()),
        node.secure_channels(),
        RemoteCredentialRetrieverInfo::create_for_project_member(
            project.authority_identifier(),
            project_authority_route,
        ),
        "test".to_string(),
    ));

    // 3. create an access control policy checking the value of the "component" attribute of the caller
    let incoming_access_control = IncomingAbac::create_name_value(
        identities().await?.identities_attributes(),
        Some(project.authority_identifier()),
        "component",
        "control",
    );
    let outgoing_access_control = OutgoingAbac::create_name_value(
        node.context(),
        identities().await?.identities_attributes(),
        Some(project.authority_identifier()),
        "component",
        "control",
    )?;

    // 4. create a tcp inlet with the above policy

    let tcp_project_route = RemoteMultiaddrResolver::default()
        .with_tcp(tcp.clone())
        .resolve(&project.route())
        .await?;
    let project_options = SecureChannelOptions::new()
        .with_credential_retriever_creator(credential_retriever)?
        .with_authority(project.authority_identifier())
        .with_trust_policy(TrustMultiIdentifiersPolicy::new(vec![project.identifier()]));

    // 4.1 first created a secure channel to the project
    let secure_channel_address = node
        .create_secure_channel(&edge_plane, tcp_project_route.route, project_options)
        .await?;
    println!("secure channel address to the project: {secure_channel_address:?}");

    // 4.3 then create a secure channel to the control node (via its relay)
    let secure_channel_listener_route = route![secure_channel_address, "forward_to_control_plane1", "untrusted"];
    let secure_channel_to_control = node
        .create_secure_channel(
            &edge_plane,
            secure_channel_listener_route.clone(),
            SecureChannelOptions::new(),
        )
        .await?;

    println!("secure channel address to the control node: {secure_channel_to_control:?}");

    // 4.4 create a TCP inlet connected to the TCP outlet on the control node
    let outlet_route = route![secure_channel_to_control, "outlet"];
    let inlet = tcp
        .create_inlet(
            "127.0.0.1:7000",
            outlet_route.clone(),
            TcpInletOptions::new()
                .with_incoming_access_control_impl(incoming_access_control)
                .with_outgoing_access_control_impl(outgoing_access_control),
        )
        .await?;
    println!("the inlet is {inlet:?}");

    // don't stop the node
    Ok(())
}
