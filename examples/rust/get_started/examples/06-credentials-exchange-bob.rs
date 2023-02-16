// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.
use hello_ockam::Echoer;
use ockam::access_control::{AbacAccessControl, AllowAll};
use ockam::authenticated_storage::AuthenticatedAttributeStorage;
use ockam::identity::authority::{AuthorityApi, AuthorityClient};
use ockam::identity::{Identity, TrustEveryonePolicy};
use ockam::{vault::Vault, Context, Result, TcpTransport, TCP};
use ockam_core::route;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport
    let tcp = TcpTransport::create(&ctx).await?;

    // Create an Identity to represent Bob
    let bob = Identity::create(&ctx, &Vault::create()).await?;

    // Create a client to the Authority
    let authority_route = route![(TCP, "127.0.0.1:5000"), "authority"];
    let authority = AuthorityClient::new(&ctx, authority_route).await?;

    // Get a credential for Bob
    let credential = authority
        .get_attribute_credential(bob.identifier(), "name", "bob")
        .await?;
    println!("got a credential from the authority\n{credential}");
    bob.set_credential(credential).await;

    // Start a worker which will receive credentials
    let authority_identity = authority.public_identity().await?;
    bob.start_credential_exchange_worker(
        vec![authority_identity],
        "credential_exchange",
        true,
        AuthenticatedAttributeStorage::new(bob.authenticated_storage().clone()),
    )
    .await?;

    // Create a TCP listener and wait for incoming connections
    tcp.listen("127.0.0.1:4000").await?;
    println!("created a TCP listener");

    // Create a secure channel listener to allow Alice to create a secure channel
    bob.create_secure_channel_listener("bob_listener", TrustEveryonePolicy)
        .await?;
    println!("created a secure channel listener");

    // Start an echoer service which will only allow subjects with name = alice
    let alice_only = AbacAccessControl::create(bob.authenticated_storage(), "name", "alice");
    ctx.start_worker("echoer", Echoer, alice_only, AllowAll).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
