use std::io;
use std::io::Write;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use ockamd::{
    cli::{
        Args,
        ChannelRole::{Initiator, Responder},
        Mode::{Control, Server},
    },
    node::{Config, Node},
    vault::FilesystemVault,
    worker::Worker,
};

use ockam_vault::software::DefaultVault;

use ockam_channel::ChannelManager;
use ockam_common::commands::ockam_commands::*;
use ockam_kex::xx::{XXInitiator, XXResponder};
use ockam_message::message::{
    AddressType, Message as OckamMessage, MessageType, Route, RouterAddress,
};
use ockam_router::router::Router;

fn main() {
    let args = Args::parse();

    match args.exec_mode() {
        Server if args.role() == Initiator => {
            let config: Config = args.into();

            // TODO: temporarily passed into the node, need to re-work
            let (router_tx, router_rx) = std::sync::mpsc::channel();
            let router = Router::new(router_rx);

            // create the vault
            // let vault = Arc::new(Mutex::new(
            //     FilesystemVault::new(config.vault_path()).expect("failed to initialize vault"),
            // ));

            let vault = Arc::new(Mutex::new(DefaultVault::default()));

            // create the channel manager
            type XXChannelManager = ChannelManager<XXInitiator, XXResponder, XXInitiator>;
            let (chan_tx, chan_rx) = mpsc::channel();
            let chan_manager =
                XXChannelManager::new(chan_rx, chan_tx, router_tx.clone(), vault).unwrap();

            // configure a node
            let node_config = config.clone();
            let (node, router_tx) =
                Node::new(None, router, router_tx.clone(), chan_manager, &node_config);

            // read from stdin, pass each line to the router within the node
            thread::spawn(move || {
                let input = io::stdin();
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
                println!(
                    "sending kex message. \nonward: {:?}\nreturn: {:?}",
                    onward_route, return_route
                );
                let kex_msg = OckamMessage {
                    message_type: MessageType::None,
                    message_body: vec![],
                    onward_route,
                    return_route,
                };
                router_tx
                    .send(OckamCommand::Router(RouterCommand::SendMessage(kex_msg)))
                    .expect("failed to send kex request message to router");

                println!("waiting for kex responder msg...");
                loop {
                    match kex_worker_rx.try_recv() {
                        Ok(cmd) => match cmd {
                            OckamCommand::Worker(WorkerCommand::ReceiveMessage(msg)) => {
                                match msg.message_type {
                                    MessageType::None => {
                                        println!("got kex responder message back: {:?}", msg);
                                        onward_route = msg.return_route.clone();
                                        break;
                                    }
                                    _ => println!("different message type: {:?}", msg),
                                }
                            }
                            _ => println!("different command: {:?}", cmd),
                        },
                        Err(e) if e == mpsc::TryRecvError::Disconnected => {
                            panic!("failed to handle kex response: {:?}", e)
                        }
                        _ => {}
                    }
                }

                let channel_addr = config
                    .channel_responder_address()
                    .expect("no channel responder address in config");

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
            });

            // run the node to poll its various internal components
            node.run();
        }
        Server if args.role() == Responder => {
            let config: Config = args.into();

            // TODO: temporarily passed into the node, need to re-work
            let (router_tx, router_rx) = std::sync::mpsc::channel();
            let router = Router::new(router_rx);

            // create the vault
            // let vault = Arc::new(Mutex::new(
            //     FilesystemVault::new(config.vault_path()).expect("failed to initialize vault"),
            // ));

            let vault = Arc::new(Mutex::new(DefaultVault::default()));

            // create the channel manager
            type XXChannelManager = ChannelManager<XXInitiator, XXResponder, XXInitiator>;
            let (chan_tx, chan_rx) = mpsc::channel();
            let chan_manager =
                XXChannelManager::new(chan_rx, chan_tx, router_tx.clone(), vault).unwrap();

            let worker_addr = RouterAddress::worker_router_address_from_str("01242020").unwrap();
            // configure a worker and node, providing it access to the vault
            let worker = Worker::new(worker_addr, router_tx.clone(), |_self, msg| {
                let mut out = std::io::stdout();
                out.write(msg.message_body.as_ref())
                    .expect("failed to write message to stdout");
                out.flush().expect("failed to flush stdout");
            });
            let (node, _) = Node::new(Some(worker), router, router_tx, chan_manager, &config);

            // run the node to poll its various internal components
            node.run();
        }
        Server => eprintln!("server mode must be executed with a role"),
        Control => unimplemented!(),
    }
}
