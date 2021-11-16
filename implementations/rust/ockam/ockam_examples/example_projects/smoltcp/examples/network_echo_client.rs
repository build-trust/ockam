#[macro_use]
extern crate tracing;

use core::str::FromStr;
use lazy_static::lazy_static;
use ockam::compat::collections::BTreeMap;
use ockam::compat::sync::Mutex;
use ockam::{Context, Result, Route};
use ockam_transport_smoltcp::{InterfaceConfiguration, SmolTcpTransport, TunTapDevice, TCP, ThreadLocalPortProvider, StdClock};
use smoltcp::iface::Routes;
use smoltcp::wire::{IpAddress, IpCidr, Ipv4Address};

fn get_peer_addr() -> (String, String) {
    let mut args = std::env::args().skip(1).take(2);

    (
        args.next().unwrap_or("192.168.69.100:10222".to_string()),
        args.next().unwrap_or("192.168.69.1".to_string()),
    )
}

lazy_static! {
    static ref DEVICE: Mutex<TunTapDevice> = Mutex::new(TunTapDevice::new("tap0").unwrap());
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let (peer_addr, bind_ip_addr) = get_peer_addr();

    let default_gateway = "192.168.69.100";

    // Configure stack
    let mut configuration = InterfaceConfiguration::<_, Routes<'static>>::new(
        [0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
        [IpCidr::new(IpAddress::from_str(&bind_ip_addr).unwrap(), 24)],
        &*DEVICE,
    );

    let mut routes = Routes::new(BTreeMap::new());
    routes
        .add_default_ipv4_route(Ipv4Address::from_str(&default_gateway).unwrap())
        .unwrap();
    configuration.set_routes(routes);

    // Initialize the TCP stack by opening a connection to a the remote
    let tcp = SmolTcpTransport::<ThreadLocalPortProvider>::create(&ctx, configuration, Some(StdClock))
        .await?;

    tcp.connect(&peer_addr).await?;

    // Send a message to the remote
    ctx.send(
        Route::new()
            .append_t(TCP, format!("{peer_addr}"))
            .append("echo_service"),
        String::from("Hello you over there!"),
    )
    .await?;

    // Then wait for a message back!
    let msg = ctx.receive::<String>().await?;
    info!("Received return message: '{msg}'");

    ctx.stop().await?;
    Ok(())
}
