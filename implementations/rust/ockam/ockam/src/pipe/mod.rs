//! Ockam pipe module

mod behavior;
pub use behavior::{BehaviorHook, PipeBehavior, ReceiverConfirm, SenderConfirm};

mod listener;
pub use listener::PipeListener;

mod receiver;
pub use receiver::PipeReceiver;

mod sender;
pub use sender::PipeSender;

#[cfg(test)]
mod tests;

use ockam_core::{Address, Result, Route};
use ockam_node::Context;

const CLUSTER_NAME: &str = "_internal.pipe";

/// Connect to the receiving end of a pipe
///
/// Returns the PipeSender's public address.
pub async fn connect_static<R: Into<Route>>(ctx: &mut Context, recv: R) -> Result<Address> {
    let addr = Address::random(0);
    PipeSender::create(
        ctx,
        recv.into(),
        addr.clone(),
        Address::random(0),
        PipeBehavior::empty(),
    )
    .await
    .map(|_| addr)
}

/// Connect to the receiving end of a pipe with custom behavior
///
/// Returns the PipeSender's public address.
pub async fn connect_static_with_behavior<R, P>(
    ctx: &mut Context,
    recv: R,
    hooks: P,
) -> Result<Address>
where
    R: Into<Route>,
    P: Into<PipeBehavior>,
{
    let addr = Address::random(0);
    PipeSender::create(
        ctx,
        recv.into(),
        addr.clone(),
        Address::random(0),
        hooks.into(),
    )
    .await
    .map(|_| addr)
}

/// Connect to the pipe receive listener and then to a pipe receiver
pub async fn connect_dynamic(_listener: Route) -> PipeSender {
    todo!()
}

/// Create a receiver with a static address
pub async fn receiver<I: Into<Address>>(ctx: &mut Context, addr: I) -> Result<()> {
    PipeReceiver::create(ctx, addr.into(), PipeBehavior::empty()).await
}

/// Create a new receiver with an explicit behavior manager
pub async fn receiver_with_behavior<A, P>(ctx: &mut Context, addr: A, b: P) -> Result<()>
where
    A: Into<Address>,
    P: Into<PipeBehavior>,
{
    PipeReceiver::create(ctx, addr.into(), b.into()).await
}

/// Create a pipe receive listener
///
/// This special worker will create pipe receivers for any incoming
/// connection.  The worker can simply be stopped via its address.
pub async fn listen_for_connections(ctx: &mut Context) -> Result<Address> {
    let addr = Address::random(0);
    PipeListener::create(ctx, addr.clone()).await.map(|_| addr)
}
