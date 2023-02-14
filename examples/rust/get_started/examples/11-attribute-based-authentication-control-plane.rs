use hello_ockam::{create_attribute_access_control, create_token, get_credentials, import_project};
use ockam::access_control::AllowAll;
use ockam::identity::authenticated_storage::{mem::InMemoryStorage, AuthenticatedAttributeStorage};
use ockam::identity::credential::{Credential, OneTimeCode};
use ockam::identity::{Identity, TrustEveryonePolicy, TrustMultiIdentifiersPolicy};

use ockam::remote::RemoteForwarder;
use ockam::{route, vault::Vault, AsyncTryClone, Context, Result, TcpTransport};
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

    // Use an in-memory storage
    let storage = InMemoryStorage::new();
    let attr_storage = AuthenticatedAttributeStorage::new(storage.clone());

    // Create an Identity for the control node
    let vault = Vault::create();
    let control_plane = Identity::create_ext(&ctx, &storage, &vault).await?;

    // 2. create a secure channel to the authority
    //    to retrieve the node credentials

    // Import the authority identity and route from the information file
    let project = import_project(project_information_path, &vault).await?;

    // create a secure channel to the authority
    // when creating the channel we check that the opposite side is indeed presenting the authority identity
    let secure_channel = control_plane
        .create_secure_channel_extended(
            project.authority_route(),
            TrustMultiIdentifiersPolicy::new(vec![project.authority_public_identifier()]),
            Duration::from_secs(120),
        )
        .await?;

    let credentials: Credential = get_credentials(&ctx, route![secure_channel, "authenticator"], token).await?;
    println!("{credentials}");

    // store the credentials and start a credentials exchange worker which will be
    // later on to exchange credentials with the edge node
    control_plane.set_credential(credentials.to_owned()).await;
    control_plane
        .start_credentials_exchange_worker(
            vec![project.authority_public_identity()],
            "credential_exchange",
            true,
            attr_storage.async_try_clone().await?,
        )
        .await?;

    // 3. create an access control policy checking the value of the "component" attribute of the caller
    let access_control: Arc<dyn IncomingAccessControl> =
        create_attribute_access_control(attr_storage.async_try_clone().await?, "component", "edge");

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
    let _ = control_plane
        .create_secure_channel_listener("untrusted", TrustEveryonePolicy)
        .await?;
    println!("created a secure channel listener");

    // don't stop the node
    Ok(())
}
