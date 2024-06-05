use ockam::identity::SecureChannelListenerOptions;
use ockam::remote::RemoteRelayOptions;
use ockam::tcp::{TcpConnectionOptions, TcpTransportExtension};
use ockam::{node, Routed, Worker};
use ockam::{Context, Result};

struct Echoer;

// Define an Echoer worker that prints any message it receives and
// echoes it back on its return route.
#[ockam::worker]
impl Worker for Echoer {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("\n[✓] Address: {}, Received: {:?}", ctx.address(), msg);

        // Echo the message body back on its return_route.
        ctx.send(msg.return_route(), msg.into_body()?).await
    }
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx).await?;
    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Start a worker, of type Echoer, at address "echoer".
    // This worker will echo back every message it receives, along its return route.
    let sc_options = SecureChannelListenerOptions::new();
    node.start_worker("echoer", Echoer).await?;
    node.flow_controls()
        .add_consumer("echoer", &sc_options.spawner_flow_control_id());

    // Create an Identity to represent Bob.
    let bob = node.create_identity().await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    node.create_secure_channel_listener(&bob, "listener", sc_options)
        .await?;

    // The computer that is running this program is likely within a private network and
    // not accessible over the internet.
    //
    // To allow Alice and others to initiate an end-to-end secure channel with this program
    // we connect with 1.node.ockam.network:4000 as a TCP client and ask the forwarding
    // service on that node to create a relay for us.
    //
    // All messages that arrive at that forwarding address will be sent to this program
    // using the TCP connection we created as a client.
    let node_in_hub = tcp
        .connect("1.node.ockam.network:4000", TcpConnectionOptions::new())
        .await?;
    let relay = node.create_relay(node_in_hub, RemoteRelayOptions::new()).await?;
    println!("\n[✓] RemoteRelay was created on the node at: 1.node.ockam.network:4000");
    println!("Forwarding address for Bob is:");
    println!("{}", relay.remote_address());

    // We won't call ctx.stop() here, this program will run until you stop it with Ctrl-C
    Ok(())
}
