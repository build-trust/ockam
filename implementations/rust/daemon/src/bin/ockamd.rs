use ockamd::{
    cli::{
        Args,
        ChannelRole::{Initiator, Responder},
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
        Server if matches!(role, Initiator) => initiator::run(cfg),
        Server if matches!(role, Responder) => responder::run(cfg),
        Server => eprintln!("server mode must be executed with a role"),
        Control => unimplemented!(),
    }
}
