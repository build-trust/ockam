use std::sync::mpsc;
use std::thread;

use ockamd::{
    cli::{
        Args,
        ChannelRole::{Initiator, Responder},
        Mode::{Control, Server},
    },
    node::{Config, Input, Node},
    worker::Worker,
};

use ockam_common::commands::ockam_commands::*;
use ockam_message::message::{Message as OckamMessage, *};

fn main() {
    let args = Args::parse();

    match args.exec_mode() {
        Server if args.role() == Initiator => run_initiator(args.into()),
        Server if args.role() == Responder => run_responder(args.into()),
        Server => eprintln!("server mode must be executed with a role"),
        Control => unimplemented!(),
    }
}

fn run_responder(config: Config) {
    let (mut node, router_tx) = Node::new(&config);

    let worker_addr = RouterAddress::worker_router_address_from_str("01242020").unwrap();
    let worker = Worker::new(worker_addr, router_tx.clone(), |_self, msg| {
        let mut out = std::io::stdout();
        out.write(msg.message_body.as_ref())
            .expect("failed to write message to stdout");
        out.flush().expect("failed to flush stdout");
    });
    // add the worker and run the node to poll its various internal components
    node.add_worker(worker);
    node.run();
}

fn run_initiator(config: Config) {
    // configure a node
    let node_config = config.clone();
    let (node, router_tx) = Node::new(&node_config);

    // read from stdin, pass each line to the router within the node
    thread::spawn(move || {
        let input = std::io::stdin();
        let mut buf = String::new();

        // create a stub worker to handle the kex messages
        let kex_worker_addr = RouterAddress::worker_router_address_from_str("01242020")
            .expect("failed to create worker address for kex");
        let (kex_worker_tx, kex_worker_rx) = mpsc::channel();
        router_tx
            .send(OckamCommand::Router(RouterCommand::Register(
                AddressType::Worker,
                kex_worker_tx,
            )))
            .expect("failed to register kex worker with router");

        // create the secure channel
        let mut onward_route = config.onward_route().expect("misconfigured onward route");
        onward_route.addresses.insert(
            0,
            RouterAddress::channel_router_address_from_str("00000000")
                .expect("failed to create zero channel address"),
        );
        let return_route = Route {
            addresses: vec![kex_worker_addr],
        };
        let kex_msg = OckamMessage {
            message_type: MessageType::None,
            message_body: vec![],
            onward_route,
            return_route,
        };
        router_tx
            .send(OckamCommand::Router(RouterCommand::SendMessage(kex_msg)))
            .expect("failed to send kex request message to router");

        // await key exchange finalization
        match kex_worker_rx.recv() {
            Ok(cmd) => match cmd {
                OckamCommand::Worker(WorkerCommand::ReceiveMessage(msg)) => {
                    match msg.message_type {
                        MessageType::None => {
                            onward_route = msg.return_route.clone();
                        }
                        _ => unimplemented!(),
                    }
                }
                _ => unimplemented!(),
            },
            Err(e) => panic!("failed to handle kex response: {:?}", e),
        }

        if matches!(config.input_kind(), Input::Stdin) {
            loop {
                if let Ok(_) = input.read_line(&mut buf) {
                    router_tx
                        .send(OckamCommand::Router(RouterCommand::SendMessage(
                            OckamMessage {
                                onward_route: onward_route.clone(),
                                return_route: Route { addresses: vec![] },
                                message_body: buf.as_bytes().to_vec(),
                                message_type: MessageType::Payload,
                            },
                        )))
                        .expect("failed to send input data to node");
                    buf.clear();
                } else {
                    eprintln!("fatal error: failed to read from input");
                    std::process::exit(1);
                }
            }
        }
    });

    // run the node to poll its various internal components
    node.run();
}
