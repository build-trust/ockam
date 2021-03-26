use crate::Context;
use ockam_core::Runner;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Sender};
use crate::relay::{RelayMessage, run_mailbox};

/// Build and spawn a new worker relay, returning a send handle to it
pub(crate) fn build<R>(rt: &Runtime, runner: &mut R, ctx: Context) -> Sender<RelayMessage>
    where
        R: Runner<Context = Context>,
{
    let (tx, rx) = channel(32);
    let mb_tx = ctx.mailbox.sender();

    runner.set_ctx(ctx);

    rt.spawn(run_mailbox(rx, mb_tx));
    tx
}
