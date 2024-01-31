use std::sync::Arc;

use hello_ockam::{create_token, import_project};
use ockam::abac::AbacAccessControl;
use ockam::identity::{
    RemoteCredentialRetriever, RemoteCredentialRetrieverInfo, SecureChannelListenerOptions, SecureChannelOptions,
    TrustMultiIdentifiersPolicy,
};
use ockam::remote::RemoteRelayOptions;
use ockam::{node, Context, Result, TcpOutletOptions, TcpTransportExtension};
use ockam_api::authenticator::enrollment_tokens::TokenAcceptor;
use ockam_api::authenticator::one_time_code::OneTimeCode;
use ockam_api::nodes::NodeManager;
use ockam_api::{multiaddr_to_route, multiaddr_to_transport_route, DefaultAddress};
use ockam_multiaddr::MultiAddr;

/// This node supports a "control" server on which several "edge" devices can connect
///
/// The connections go through the Ockam Orchestrator, via a Relay, and a secure channel
/// can be established to forward messages to an outlet going to a local Python webserver.
///
///
/// This example shows how to:
///
///   - retrieve credentials from an authority
///   - create a Relay on the Ockam Orchestrator
///   - create a TCP outlet with some access control checking the authenticated attributes of the caller
///
/// The node needs to be started with:
///
///  - a project.json file created with `ockam project information --output json  > project.json`
///  - a token created by an enroller node with `ockam project ticket --attribute component=control`
///
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // create a token (with `ockam project ticket --attribute component=control`)
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
    let node = node(ctx).await?;
    // Initialize the TCP transport
    let tcp = node.create_tcp_transport().await?;

    // Create an Identity for the control node
    let control_plane = node.create_identity().await?;

    // 2. create a secure channel to the authority node
    //    to retrieve the node credential
    // create a secure channel to the authority
    // when creating the channel we check that the opposite side is indeed presenting the authority identity
    let authority_node = NodeManager::authority_node_client(
        &tcp,
        node.secure_channels().clone(),
        &control_plane,
        &MultiAddr::try_from("/dnsaddr/localhost/tcp/5000")?,
        &node.create_identity().await?,
    )
    .await?;
    authority_node.present_token(node.context(), token).await.unwrap();

    let project = import_project(project_information_path, node.identities()).await?;
    let project_authority_route = multiaddr_to_transport_route(&project.authority_route()).unwrap(); // FIXME: Handle error

    // Create a credential retriever that will be used to obtain credentials
    let credential_retriever = Arc::new(RemoteCredentialRetriever::new(
        Arc::new(tcp.clone()),
        node.secure_channels(),
        RemoteCredentialRetrieverInfo::new(
            project.authority_identifier(),
            project_authority_route,
            DefaultAddress::CREDENTIAL_ISSUER.into(),
        ),
    ));

    // 3. create an access control policy checking the value of the "component" attribute of the caller
    let access_control = AbacAccessControl::create(
        node.identities_attributes(),
        project.authority_identifier(),
        "component",
        "edge",
    );

    // 4. create a tcp outlet with the above policy
    tcp.create_outlet(
        "outlet",
        "127.0.0.1:5000",
        TcpOutletOptions::new().with_incoming_access_control_impl(access_control),
    )
    .await?;

    // 5. create a relay on the Ockam orchestrator

    let tcp_project_route = multiaddr_to_route(&project.route(), &tcp).await.unwrap(); // FIXME: Handle error
    let project_options = SecureChannelOptions::new()
        .with_credential_retriever(credential_retriever)?
        .with_authority(project.authority_identifier())
        .with_trust_policy(TrustMultiIdentifiersPolicy::new(vec![project.identifier()]));

    // create a secure channel to the project first
    // we make sure that we indeed connect to the correct project on the Orchestrator
    let secure_channel_address = node
        .create_secure_channel(&control_plane, tcp_project_route.route, project_options)
        .await?;
    println!("secure channel to project: {secure_channel_address:?}");

    // finally create a relay using the secure channel to the project
    let relay = node
        .create_static_relay(secure_channel_address, "control_plane1", RemoteRelayOptions::new())
        .await?;
    println!("relay is {relay:?}");

    // 6. create a secure channel listener which will allow the edge node to
    //    start a secure channel when it is ready
    node.create_secure_channel_listener(&control_plane, "untrusted", SecureChannelListenerOptions::new())
        .await?;
    println!("created a secure channel listener");

    // don't stop the node
    Ok(())
}
