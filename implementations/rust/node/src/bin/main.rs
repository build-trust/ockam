use ockam_message::message::Address;
use ockam_node::node::{start_node, Role};
use std::net::SocketAddr;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(author = "Ockam Developers (ockam.io)")]
pub struct Args {
    /// Local socket (initiator and/or responder)
    #[structopt(long = "local-udp")]
    local_addr_udp: Option<String>,

    /// Tcp router (responder)
    #[structopt(long = "router-tcp")]
    router_addr_tcp: Option<String>,

    /// Remote address to send to
    #[structopt(long = "remote-socket")]
    remote_addr: Option<String>,

    /// Worker
    #[structopt(short = "w", long = "worker")]
    worker_addr: Option<String>,

    #[structopt(long = "role", required = true)]
    role: String,

    #[structopt(long = "tcp-listen")]
    listen_addr: Option<String>,
}

// return value:
// - local socket (udp only)
// - router socket
// - remote (sink) socket
// - remote worker address
// - tcp listen address
// - initiator ip protocol
pub fn parse_args(
    args: Args,
) -> Result<
    (
        Option<SocketAddr>,
        Option<SocketAddr>,
        Option<SocketAddr>,
        Option<Address>,
        Option<SocketAddr>,
        Role,
    ),
    String,
> {
    let role = args.role();

    // local udp socket
    let local_udp;
    if let Some(lua) = args.local_addr_udp {
        if let Ok(sock_addr) = SocketAddr::from_str(&lua) {
            local_udp = Some(sock_addr);
        } else {
            return Err("error parsing local udp address".into());
        }
    } else {
        local_udp = None;
    }

    // router address
    let router_addr;
    if let Some(ra) = args.router_addr_tcp {
        router_addr = Some(SocketAddr::from_str(&ra).unwrap());
    } else {
        router_addr = None;
    }

    // remote address
    let remote_addr;
    if let Some(ra) = args.remote_addr {
        remote_addr = Some(SocketAddr::from_str(&ra).unwrap());
    } else {
        remote_addr = None;
    }

    // worker address
    let mut worker_addr = Some(Address::WorkerAddress(vec![0, 0, 0, 0]));
    if let Some(worker_addr_str) = args.worker_addr {
        worker_addr = Some(Address::worker_address_from_string(&worker_addr_str).unwrap());
    }

    // listen address
    let listen_addr;
    if let Some(listen) = args.listen_addr {
        listen_addr = Some(SocketAddr::from_str(&listen).unwrap());
    } else {
        listen_addr = None;
    }

    Ok((
        local_udp,
        router_addr,
        remote_addr,
        worker_addr,
        listen_addr,
        role,
    ))
}

impl Args {
    pub fn role(&self) -> Role {
        if self.role == "source" {
            return Role::Source;
        }
        if self.role == "sink" {
            return Role::Sink;
        }
        if self.role == "hub" || self.role == "router" {
            return Role::Hub;
        }
        if self.role == "blaster" {
            return Role::Blaster;
        }
        if self.role == "blastee" {
            return Role::Blastee;
        }
        panic!("invalid role specified");
    }
}

fn main() {
    let args = Args::from_args();
    println!("{:?}", args);
    let local_udp: Option<SocketAddr>;
    let router_addr: Option<SocketAddr>;
    let remote_addr: Option<SocketAddr>;
    let worker_addr: Option<Address>;
    let listen_addr: Option<SocketAddr>;
    let _router_only: bool;
    let role: Role;
    match parse_args(args) {
        Ok((local, router, remote, worker, listen, r)) => {
            local_udp = local;
            router_addr = router;
            remote_addr = remote;
            worker_addr = worker;
            listen_addr = listen;
            role = r;
        }
        Err(s) => {
            println!("{}", s);
            return;
        }
    }

    match start_node(
        local_udp,
        router_addr,
        remote_addr,
        worker_addr,
        listen_addr,
        role,
    ) {
        Err(s) => {
            println!("{}", s);
        }
        _ => {}
    }
}
