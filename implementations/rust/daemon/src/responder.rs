use std::io::Write;

use crate::config::Config;
use crate::node::Node;
use crate::worker::Worker;

use ockam_message::message::RouterAddress;

pub fn run(config: Config) {
    // 1. generate key pair
    // 2. print the public key
    // 3.

    let (mut node, router_tx) = Node::new(&config);

    let worker_addr = RouterAddress::worker_router_address_from_str("01242020").unwrap();
    let worker = Worker::new(worker_addr, router_tx, |_self, msg| {
        let mut out = std::io::stdout();
        out.write_all(msg.message_body.as_ref())
            .expect("failed to write message to stdout");
        out.flush().expect("failed to flush stdout");
    });
    // add the worker and run the node to poll its various internal components
    node.add_worker(worker);
    node.run();
}
