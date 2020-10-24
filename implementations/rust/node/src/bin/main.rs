use ockam_message::message::{Address, AddressType, Route, RouterAddress};
use ockam_node::node::node::start_node;
use std::net::SocketAddr;
use std::str::FromStr;
use std::{thread, time};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(author = "Ockam Developers (ockam.io)")]
pub struct Args {
    /// Local address to bind socket
    #[structopt(short = "l", long = "local")]
    local_socket: Option<String>,

    /// Remote address to send to
    #[structopt(short = "r", long = "remote")]
    remote_socket: Option<String>,

    /// Via - intermediate router
    #[structopt(short = "v", long = "via")]
    via_socket: Option<String>,

    /// Worker
    #[structopt(short = "w", long = "worker")]
    worker_addr: Option<String>,
}

pub fn parse_args(args: Args) -> Result<(RouterAddress, Route, RouterAddress), String> {
    let mut local_socket: RouterAddress = RouterAddress {
        a_type: AddressType::Udp,
        length: 7,
        address: (Address::UdpAddress(SocketAddr::from_str("127.0.0.1:4050").unwrap())),
    };
    if let Some(l) = args.local_socket {
        if let Ok(sa) = SocketAddr::from_str(&l) {
            if let Some(ra) = RouterAddress::from_address(Address::UdpAddress(sa)) {
                local_socket = ra;
            }
        }
    } else {
        return Err("local socket address required: -l xxx.xxx.xxx.xxx:pppp".to_string());
    }

    let mut route = Route { addresses: vec![] };

    if let Some(vs) = args.via_socket {
        if let Ok(sa) = SocketAddr::from_str(&vs) {
            if let Some(ra) = RouterAddress::from_address(Address::UdpAddress(sa)) {
                route.addresses.push(ra);
            }
        }
    };

    let mut worker_address = RouterAddress::worker_router_address_from_str("00000000").unwrap();
    if let Some(rs) = args.remote_socket {
        if let Ok(sa) = SocketAddr::from_str(&rs) {
            if let Some(ra) = RouterAddress::from_address(Address::UdpAddress(sa)) {
                route.addresses.push(ra);
                if let Some(wa) = args.worker_addr {
                    worker_address = RouterAddress::worker_router_address_from_str(&wa).unwrap();
                }
            }
        }
    };

    Ok((local_socket, route, worker_address))
}

fn main() {
    let args = Args::from_args();
    println!("{:?}", args);
    let local_socket: RouterAddress;
    let route: Route;
    let worker: RouterAddress;
    match parse_args(args) {
        Ok((ls, r, w)) => {
            local_socket = ls;
            route = r;
            worker = w;
        }
        Err(s) => {
            println!("{}", s);
            return;
        }
    }
    let is_initiator = !route.addresses.is_empty();

    println!("route: {:?}", route);

    start_node(local_socket, route, worker, is_initiator);

    thread::sleep(time::Duration::from_millis(1000000));
}
