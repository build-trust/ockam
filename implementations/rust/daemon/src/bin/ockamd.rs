use std::io::stdin;

use ockam_vault::software::DefaultVault as FilesystemVault;

use ockamd::{cli, node::Node};

fn main() {
    let args = cli::Args::parse();

    match args.exec_mode() {
        cli::Mode::Server => {
            let config = args.into();
            let mut vault = FilesystemVault::default();

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
        cli::Mode::Control => unimplemented!(),
    }
}
