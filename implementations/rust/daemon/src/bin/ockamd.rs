use ockamd::config::Config;
use ockamd::initiator::StdinWorker;
use ockamd::node::{Node, OckamdWorker};
use ockamd::worker::Worker;
use ockamd::{
    cli::{
        Args,
        ChannelRole::{Router, Sink, Source},
        Mode::{Control, Server},
    },
    initiator, responder,
};

fn main() {
    let args = Args::parse();
    let role = args.role();
    let mode = args.exec_mode();
    let config = args.into();

    let (mut node, router_tx) = Node::new(&config);

    // match mode {
    //     Server if matches!(role, Source) => {
    //         souce_worker = initiator::initialize(cfg, router_tx.clone());
    //         //            node.add_worker()
    //     }
    // Server if matches!(role, Sink) => responder::run(cfg),
    // Server if matches!(role, Router) => responder::run(cfg),
    // Server => eprintln!("server mode must be executed with a role"),
    // Control => unimplemented!(),
    //     _ => {}
    // }
    node.run();
}
