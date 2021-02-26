use crate::{relay, Context, Executor, Mailbox, NodeMessage};
use ockam_core::Address;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Sender};

pub struct App;

impl ockam_core::Worker for App {
    type Context = Context;
    type Message = (); // This message type is never used
}

pub fn start_node() -> (Context, Executor) {
    let mut exe = Executor::new();
    let addr = "app".into();

    // The root application worker needs a mailbox and relay to accept
    // messages from workers, and to buffer incoming transcoded data.
    let ctx = root_app_context(exe.runtime(), &addr, exe.sender());

    // Build a mailbox worker to buffer messages
    let sender = relay::build_root::<App, _>(exe.runtime(), &ctx.mailbox);

    // Register this mailbox handle with the executor
    exe.initialize_system("app", sender);

    (ctx, exe)
}

fn root_app_context(rt: Arc<Runtime>, addr: &Address, tx: Sender<NodeMessage>) -> Context {
    let (mb_tx, mb_rx) = channel(32);
    let mb = Mailbox::new(mb_rx, mb_tx.clone());
    let ctx = Context::new(rt, tx, addr.into(), mb);
    ctx
}
