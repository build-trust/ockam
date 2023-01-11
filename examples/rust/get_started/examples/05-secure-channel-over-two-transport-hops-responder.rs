// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::authenticated_storage::InMemoryStorage;
use ockam::identity::{Identity, SecureChannelRegistry, TrustEveryonePolicy};
use ockam::{vault::Vault, Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    // Create a Vault to safely store secret keys for Bob.
    let vault = Vault::create();

    // Create a SecureChannel registry.
    let registry = SecureChannelRegistry::new();

    // Create an Identity to represent Bob.
    let bob = Identity::create(&ctx, &vault).await?;

    // Create an AuthenticatedStorage to store info about Bob's known Identities.
    let storage = InMemoryStorage::new();

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    bob.create_secure_channel_listener("bob_listener", TrustEveryonePolicy, &storage, &registry)
        .await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
