// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a tcp listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use ockam::access_control::AllowedTransport;
use ockam::{Address, Context, Mailboxes, Result, TcpTransport, TCP};
use std::sync::Arc;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // FIXME: Child context is needed because "app" Context has LocalOriginOnly AccessControl,
    // that can be fixed by improving #[ockam::node] macro
    let child_ctx = ctx
        .new_context_impl(Mailboxes::main(
            Address::random_local(),
            Arc::new(AllowedTransport::single(TCP)),
        ))
        .await?;

    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&child_ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:3000").await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
