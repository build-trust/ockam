use hello_ockam::{create_attribute_access_control, create_token, get_credentials, import_project, OneTimeCode};
use ockam::access_control::AllowAll;
use ockam::identity::credential::Credential;
use ockam::identity::{Identity, TrustEveryonePolicy, TrustMultiIdentifiersPolicy};
use ockam::remote::RemoteForwarder;
use ockam::{route, vault::Vault, Context, Result, TcpTransport};
use ockam_core::IncomingAccessControl;
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
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP transport
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an Identity for the control node
    let vault = Vault::create();
    let control_plane = Identity::create(&ctx, &vault).await?;

    // 1. create a secure channel to the authority
    //    to retrieve the node credentials

    // Import the authority identity and route from the project.json file
    let project = import_project("project.json", &vault).await?;

    // create a secure channel to the authority
    // when creating the channel we check that the opposite side is indeed presenting the authority identity
    let secure_channel = control_plane
        .create_secure_channel_extended(
            project.authority_route.clone(),
            TrustMultiIdentifiersPolicy::new(vec![project.authority_public_identifier()]),
            Duration::from_secs(120),
        )
        .await?;

    // 2. get credentials using a one-time token

    // create the token obtained with `ockam project enroll --attribute component=control`
    // you can also copy and paste a token here and parse it with the project/otc_parser function
    let token: OneTimeCode = create_token("component", "control").await?;
    println!("token: {token:?}");

    let credentials: Credential = get_credentials(&ctx, route![secure_channel, "authenticator"], token).await?;
    println!("{credentials}");

    // store the credentials and start a credentials exchange worker which will be
    // later on to exchange credentials with the edge node
    control_plane.set_credential(Some(credentials.to_owned())).await;
    control_plane
        .start_credentials_exchange_worker(vec![project.authority_public_identity()], "credential_exchange", true)
        .await?;

    // 3. create an access control policy checking the value of the "component" attribute of the caller
    let access_control: Arc<dyn IncomingAccessControl> =
        create_attribute_access_control(control_plane.authenticated_storage().clone(), "component", "edge");

    // 4. create a tcp outlet with the above policy
    let outlet = tcp
        .create_outlet_impl("outlet".into(), "127.0.0.1:5000".into(), access_control)
        .await?;
    println!("{outlet:?}");

    // 5. create a forwarder on the Ockam orchestrator

    // create a secure channel to the project first
    // we make sure that we indeed connect to the correct project on the Orchestrator
    let secure_channel_address = control_plane
        .create_secure_channel_extended(
            project.route(),
            TrustMultiIdentifiersPolicy::new(vec![project.identifier()]),
            Duration::from_secs(120),
        )
        .await?;
    println!("secure channel to project: {secure_channel_address:?}");

    // present this node credentials to the project
    control_plane
        .present_credential(route![secure_channel_address.clone(), "credentials"])
        .await?;

    // finally create a forwarder using the secure channel to the project
    let forwarder = RemoteForwarder::create_static(&ctx, secure_channel_address, "control_plane1", AllowAll).await?;
    println!("forwarder is {forwarder:?}");

    // 6. create a secure channel listener which will allow the edge node to
    //    start a secure channel when it is ready
    let secure_channel_listener = control_plane
        .create_secure_channel_listener("untrusted", TrustEveryonePolicy)
        .await?;
    println!("listener is {secure_channel_listener:?}");

    // don't stop the node
    Ok(())
}
