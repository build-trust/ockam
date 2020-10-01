use std::io::stdin;

use ockamd::{
    cli::{
        Args,
        ChannelRole::{Initiator, Responder},
        Mode::{Control, Server},
    },
    node::{Config, Node},
    vault::FilesystemVault,
};

fn main() {
    let args = Args::parse();

    match args.exec_mode() {
        Server if args.role() == Initiator => {
            let config: Config = args.into();
            let vault =
                FilesystemVault::new(config.vault_path()).expect("failed to initialize vault");

            // create the server using the input type and get encrypted message from server
            let tx_input = Node::new(vault, config);

            // using stdin as example input
            let input = stdin();
            let mut buf = String::new();
            loop {
                if let Ok(_n) = input.read_line(&mut buf) {
                    tx_input
                        .send(buf.as_bytes().to_vec())
                        .expect("failed to send input data to node");
                    buf.clear();
                }
            }
        }
        Server if args.role() == Responder => unimplemented!(),
        Server => eprintln!("server mode must be executed with a role"),
        Control => unimplemented!(),
    }
}
