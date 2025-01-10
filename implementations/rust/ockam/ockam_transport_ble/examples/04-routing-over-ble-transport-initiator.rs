// This node routes a message, to a worker on a different node, over the ble transport.

use ockam_core::{route, Result};
use ockam_node::Context;

use ockam_transport_ble::driver::btleplug::BleAdapter;
use ockam_transport_ble::{BleClient, BleTransport, BLE};

fn main() -> Result<()> {
    let (ctx, mut exe) = ockam_node::NodeBuilder::new().build();
    exe.execute(async move { async_main(ctx).await })
        .unwrap()
        .unwrap();
    Ok(())
}

async fn async_main(mut ctx: Context) -> Result<()> {
    // Create a ble_client
    let ble_adapter = BleAdapter::try_new().await?;
    let ble_client = BleClient::with_adapter(ble_adapter);

    // Initialize the BLE Transport.
    let ble = BleTransport::create(&ctx)?;

    // Try to connect to BleServer
    ble.connect(ble_client, "ockam_ble_1".to_string()).await?;

    // Send a message to the "echoer" worker, on a different node, over a ble transport.
    let r = route![(BLE, "ockam_ble_1"), "echoer"];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("[main] App Received: {}", reply.into_body()?); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.shutdown_node().await
}
