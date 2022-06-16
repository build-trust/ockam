// This node routes a message, to a worker on a different node, over the tcp transport.

use ockam::abac::{AbacLocalInfo, AbacWrapperWorker, Action, Resource, Subject};
use ockam::access_control::{AllowedTransport, LocalOriginOnly};
use ockam::{route, Context, Result, TcpTransport, WorkerBuilder, TCP};

#[ockam::node(access_control = "LocalOriginOnly")]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an abac wrapper worker
    WorkerBuilder::with_access_control(
        LocalOriginOnly,
        "abac_wrapper",
        AbacWrapperWorker::new(route![(TCP, "localhost:4000"), "abac_unwrapper"]),
    )
    .start(&ctx)
    .await?;

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    // Create a message
    let message = "Hello Ockam!".to_string();

    // Create AbacLocalInfo for request
    let local_info = AbacLocalInfo::new(
        Subject::from(0x0000_0000_0000_0001),
        Resource::from("/project/green/1234"),
        Action::from("read"),
    )
    .try_into()?;

    // A repeater Context is needed because the node Context has
    // LocalOriginOnly AccessControl.
    let mut repeater_ctx = ctx.new_repeater(AllowedTransport::single(TCP)).await?;

    // Send a message to the "echoer" worker, on a different node,
    // through an abac envelope wrapper and over a tcp transport.
    let route = route!["abac_wrapper", "echoer"];
    repeater_ctx
        .send_with_local_info(route, message, vec![local_info])
        .await?;

    // Wait to receive a reply and print it.
    let reply = repeater_ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
