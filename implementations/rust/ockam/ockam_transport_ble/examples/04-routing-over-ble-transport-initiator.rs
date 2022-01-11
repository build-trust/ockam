// This node routes a message, to a worker on a different node, over the ble transport.

use ockam::{route, Context, Result};
use ockam_transport_ble::{BleClient, BleTransport, BLE};

use ockam_transport_ble::driver::btleplug::BleAdapter;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a ble_client
    let ble_adapter = BleAdapter::try_new().await?;
    let ble_client = BleClient::with_adapter(ble_adapter);

    // Initialize the BLE Transport.
    let ble = BleTransport::create(&ctx).await?;

    // Try to connect to BleServer
    ble.connect(ble_client, "ockam_ble_1".to_string()).await?;

    // Send a message to the "echoer" worker, on a different node, over a ble transport.
    let r = route![(BLE, "ockam_ble_1"), "echoer"];
    ctx.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("[main] App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
