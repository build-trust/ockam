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
    let cfg = args.into();

    match mode {
        Server if matches!(role, Source) => initiator::run(cfg),
        Server if matches!(role, Sink) => responder::run(cfg),
        Server if matches!(role, Router) => responder::run(cfg),
        Server => eprintln!("server mode must be executed with a role"),
        Control => unimplemented!(),
    }
}
