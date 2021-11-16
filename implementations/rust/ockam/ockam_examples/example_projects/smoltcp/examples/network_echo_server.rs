//! This example is part of `network_echo`
//!
//! You need to start this binary first, before letting the
//! `network_echo_client` connect to it.

#[macro_use]
extern crate tracing;

use core::str::FromStr;
use lazy_static::lazy_static;
use ockam::compat::sync::Mutex;
use ockam::{Context, Result, Routed, Worker};
use ockam_transport_smoltcp::{InterfaceConfiguration, SmolTcpTransport, TunTapDevice, ThreadLocalPortProvider, StdClock};
use smoltcp::iface::Routes;
use smoltcp::wire::{IpAddress, IpCidr};

struct Responder;

#[ockam::worker]
impl Worker for Responder {
    type Context = Context;
    type Message = String;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        info!("Responder: {}", msg);
        ctx.send(msg.return_route(), msg.body()).await?;
        Ok(())
    }
}

fn get_bind_addr() -> String {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        .unwrap_or(format!("192.168.69.1:10222"))
}

lazy_static! {
    static ref DEVICE: Mutex<TunTapDevice> = Mutex::new(TunTapDevice::new("tap0").unwrap());
}

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Get either the default socket address, or a user-input
    let bind_addr = get_bind_addr();
    debug!("Binding to: {}", bind_addr);

    let (bind_ip_addr, bind_port) = match bind_addr.split(":").collect::<Vec<&str>>()[..] {
        [bind_ip_addr, bind_port, ..] => (bind_ip_addr, bind_port.parse().unwrap()),
        _ => panic!("Cannot parse address"),
    };

    let configuration = InterfaceConfiguration::<_, Routes<'static>>::new(
        [0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
        [IpCidr::new(IpAddress::from_str(bind_ip_addr).unwrap(), 24)],
        &*DEVICE,
    );
    let tcp = SmolTcpTransport::<ThreadLocalPortProvider>::create(&ctx, configuration, Some(StdClock))
        .await?;

    tcp.listen(bind_port).await?;

    // Create the responder worker
    ctx.start_worker("echo_service", Responder).await?;

    // The server never shuts down
    Ok(())
}
