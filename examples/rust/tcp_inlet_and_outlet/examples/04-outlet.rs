use ockam::flow_control::{FlowControlPolicy, FlowControls};
use ockam::identity::SecureChannelListenerOptions;
use ockam::remote::RemoteForwarderOptions;
use ockam::{node, Context, Result, TcpConnectionOptions, TcpOutletOptions};
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    let e = node.create_identity().await?;
    let secure_channel_flow_control_id = FlowControls::generate_id();
    let tcp_flow_control_id = FlowControls::generate_id();
    node.create_secure_channel_listener(
        &e,
        "secure_channel_listener",
        SecureChannelListenerOptions::new(&secure_channel_flow_control_id)
            .as_consumer(&tcp_flow_control_id, FlowControlPolicy::ProducerAllowMultiple),
    )
    .await?;

    // Expect first command line argument to be the TCP address of a target TCP server.
    // For example: 127.0.0.1:4002
    //
    // Create a TCP Transport Outlet - at Ockam Worker address "outlet" -
    // that will connect, as a TCP client, to the target TCP server.
    //
    // This Outlet will:
    // 1. Unwrap the payload of any Ockam Routing Message that it receives from an Inlet
    //    and send it as raw TCP data to the target TCP server. First such message from
    //    an Inlet is used to remember the route back the Inlet.
    //
    // 2. Wrap any raw TCP data it receives, from the target TCP server,
    //    as payload of a new Ockam Routing Message. This Ockam Routing Message will have
    //    its onward_route be set to the route to an Inlet that is knows about because of
    //    a previous message from the Inlet.

    let outlet_target = std::env::args().nth(1).expect("no outlet target given");
    tcp.create_outlet(
        "outlet",
        outlet_target,
        TcpOutletOptions::new().as_consumer(
            &secure_channel_flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        ),
    )
    .await?;

    // To allow Inlet Node and others to initiate an end-to-end secure channel with this program
    // we connect with 1.node.ockam.network:4000 as a TCP client and ask the forwarding
    // service on that node to create a forwarder for us.
    //
    // All messages that arrive at that forwarding address will be sent to this program
    // using the TCP connection we created as a client.
    let node_in_hub = tcp
        .connect(
            "1.node.ockam.network:4000",
            TcpConnectionOptions::as_producer(&tcp_flow_control_id),
        )
        .await?;
    let forwarder = node
        .create_forwarder(node_in_hub, RemoteForwarderOptions::new())
        .await?;
    println!("\n[✓] RemoteForwarder was created on the node at: 1.node.ockam.network:4000");
    println!("Forwarding address in Hub is:");
    println!("{}", forwarder.remote_address());

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}
