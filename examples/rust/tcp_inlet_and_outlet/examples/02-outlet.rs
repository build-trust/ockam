use ockam::flow_control::{FlowControlPolicy, FlowControls};
use ockam::{node, Context, Result, TcpListenerOptions, TcpOutletOptions};
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

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

    let tcp_flow_control_id = FlowControls::generate_id();

    let outlet_target = std::env::args().nth(1).expect("no outlet target given");
    tcp.create_outlet(
        "outlet",
        outlet_target,
        TcpOutletOptions::new().as_consumer(&tcp_flow_control_id, FlowControlPolicy::SpawnerAllowMultipleMessages),
    )
    .await?;

    // Create a TCP listener to receive Ockam Routing Messages from other ockam nodes.
    //
    // Use port 4000, unless otherwise specified by second command line argument.

    let port = std::env::args().nth(2).unwrap_or_else(|| "4000".to_string());
    tcp.listen(
        format!("127.0.0.1:{port}"),
        TcpListenerOptions::new(&tcp_flow_control_id),
    )
    .await?;

    // We won't call ctx.stop() here,
    // so this program will keep running until you interrupt it with Ctrl-C.
    Ok(())
}
