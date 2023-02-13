use hello_ockam::{create_attribute_access_control, create_token, get_credentials, import_project, OneTimeCode};
use ockam::identity::credential::Credential;
use ockam::identity::{Identity, TrustEveryonePolicy, TrustMultiIdentifiersPolicy};
use ockam::{route, vault::Vault, Context, Result, TcpTransport};
use ockam_core::IncomingAccessControl;
use std::env;
use std::sync::Arc;
use std::time::Duration;

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
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Use the TCP transport
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an Identity for the edge plane
    let vault = Vault::create();
    let edge_plane = Identity::create(&ctx, &vault).await?;

    // 1. create a secure channel to the authority
    //    to retrieve the node credentials

    // Import the authority identity and route from the project.json file
    let project = import_project("project.json", &vault).await?;

    // create a secure channel to the authority
    // when creating the channel we check that the opposite side is indeed presenting the authority identity
    let secure_channel = edge_plane
        .create_secure_channel_extended(
            project.authority_route(),
            TrustMultiIdentifiersPolicy::new(vec![project.authority_public_identifier()]),
            Duration::from_secs(120),
        )
        .await?;

    // 2. get credentials using a one-time token

    // create the token obtained with `ockam project enroll --attribute component=edge`
    // you can also copy and paste a token here and parse it with the project/otc_parser function
    let token: OneTimeCode = create_token("component", "edge").await?;
    println!("token: {token:?}");

    let credentials: Credential = get_credentials(&ctx, route![secure_channel, "authenticator"], token).await?;
    println!("{credentials}");

    // store the credentials and start a credentials exchange worker which will be
    // later on to exchange credentials with the control node
    edge_plane.set_credential(Some(credentials.to_owned())).await;
    edge_plane
        .start_credentials_exchange_worker(vec![project.authority_public_identity()], "credential_exchange", true)
        .await?;

    // 3. create an access control policy checking the value of the "component" attribute of the caller
    let access_control: Arc<dyn IncomingAccessControl> =
        create_attribute_access_control(edge_plane.authenticated_storage().clone(), "component", "control");

    // 4. create a tcp inlet with the above policy

    // 4.1 first created a secure channel to the project
    let secure_channel_address = edge_plane
        .create_secure_channel_extended(
            project.route(),
            TrustMultiIdentifiersPolicy::new(vec![project.identifier()]),
            Duration::from_secs(120),
        )
        .await?;
    println!("secure channel address to the project: {secure_channel_address:?}");

    // 4.2 and send this node credentials to the project
    edge_plane
        .present_credential(route![secure_channel_address.clone(), "credentials"])
        .await?;

    // 4.3 then create a secure channel to the control node (via its forwarder)
    let secure_channel_listener_route = route![secure_channel_address, "forward_to_control_plane1", "untrusted"];
    let secure_channel_to_control = edge_plane
        .create_secure_channel_extended(
            secure_channel_listener_route.clone(),
            TrustEveryonePolicy,
            Duration::from_secs(120),
        )
        .await?;

    println!("secure channel address to the control node: {secure_channel_to_control:?}");

    // 4.4 exchange credentials with the control node
    edge_plane
        .present_credential_mutual(
            route![secure_channel_to_control.clone(), "credential_exchange"],
            vec![&project.authority_public_identity()],
        )
        .await?;
    println!("credential exchange done");

    // 4.5 create a TCP inlet connected to the TCP outlet on the control node
    let outlet_route = route![secure_channel_to_control, "outlet"];
    let inlet = tcp
        .create_inlet_impl("127.0.0.1:7000".into(), outlet_route.clone(), access_control)
        .await?;
    println!("the inlet is {inlet:?}");

    // don't stop the node
    Ok(())
}
