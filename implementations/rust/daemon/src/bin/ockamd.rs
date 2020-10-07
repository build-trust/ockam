use std::io;
use std::io::Write;
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

use ockam_common::commands::ockam_commands::*;
use ockam_message::message::{Message as OckamMessage, MessageType, Route};

fn main() {
    let args = Args::parse();

    match args.exec_mode() {
        Server if args.role() == Initiator => {
            let config: Config = args.into();
            let vault =
                FilesystemVault::new(config.vault_path()).expect("failed to initialize vault");

            // configure a node, providing it access to the vault
            let (node, router_tx) = Node::new(vault, None, &config);

            // read from stdin, pass each line to the router within the node
            thread::spawn(move || {
                let input = io::stdin();
                let mut buf = String::new();
                loop {
                    if let Ok(_n) = input.read_line(&mut buf) {
                        router_tx
                            .send(OckamCommand::Router(RouterCommand::SendMessage(
                                OckamMessage {
                                    onward_route: config
                                        .onward_route()
                                        .expect("miconfigured onward route"),
                                    return_route: Route { addresses: vec![] },
                                    message_body: buf.as_bytes().to_vec(),
                                    message_type: MessageType::Payload,
                                },
                            )))
                            .expect("failed to send input data to node");
                        buf.clear();
                    } else {
                        eprintln!("fatal error: failed to read from inpput");
                        std::process::exit(1);
                    }
                }
            });

            // run the node to poll its various internal components
            node.run();
        }
        Server if args.role() == Responder => {
            let config: Config = args.into();
            let vault =
                FilesystemVault::new(config.vault_path()).expect("failed to initialize vault");

            // configure a worker and node, providing it access to the vault
            let worker = Worker::new(|msg, _router_tx| {
                println!("were in the worker!!!!");
                let mut out = std::io::stdout();
                out.write(msg.message_body.as_ref())
                    .expect("failed to write message to stdout");
                out.flush().expect("failed to flush stdout");
            });
            let (node, _) = Node::new(vault, Some(worker), &config);

            // run the node to poll its various internal components
            node.run();
        }
        Server => eprintln!("server mode must be executed with a role"),
        Control => unimplemented!(),
    }
}
