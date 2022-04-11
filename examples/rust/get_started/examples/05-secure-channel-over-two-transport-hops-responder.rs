// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowedTransport;
use ockam::identity::{Identity, TrustEveryonePolicy};
use ockam::{vault::Vault, Address, Context, Mailboxes, Result, TcpTransport, TCP};
use std::sync::Arc;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker("echoer", Echoer).await?;

    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    // Create a Vault to safely store secret keys for Bob.
    let vault = Vault::create();

    // FIXME: Child context is needed because "app" Context has LocalOriginOnly AccessControl,
    // that can be fixed by improving #[ockam::node] macro
    let child_ctx = ctx
        .new_context_impl(Mailboxes::main(
            Address::random_local(),
            Arc::new(AllowedTransport::single(TCP)),
        ))
        .await?;

    // Create an Identity to represent Bob.
    let bob = Identity::create(&child_ctx, &vault).await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    bob.create_secure_channel_listener("bob_listener", TrustEveryonePolicy)
        .await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
