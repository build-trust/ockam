//! Ockam pipe module

mod behavior;
pub use behavior::{BehaviorHook, PipeBehavior, ReceiverConfirm, SenderConfirm};

mod listener;
pub use listener::PipeListener;

mod receiver;
pub use receiver::PipeReceiver;

mod sender;
pub use sender::PipeSender;

use crate::protocols::pipe::{internal::InternalCmd, PipeMessage};
use ockam_core::{async_trait, Address, Result, Route};
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
pub async fn connect_static_with_behavior<R: Into<Route>>(
    ctx: &mut Context,
    recv: R,
    hooks: PipeBehavior,
) -> Result<Address> {
    let addr = Address::random(0);
    PipeSender::create(ctx, recv.into(), addr.clone(), Address::random(0), hooks)
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
pub async fn receiver_with_behavior<I: Into<Address>>(
    ctx: &mut Context,
    addr: I,
    b: PipeBehavior,
) -> Result<()> {
    PipeReceiver::create(ctx, addr.into(), b).await
}

/// Create a pipe receive listener
///
/// This special worker will create pipe receivers for any incoming
/// connection.  The worker can simply be stopped via its address.
pub async fn listen_for_connections(ctx: &mut Context) -> Result<Address> {
    let addr = Address::random(0);
    PipeListener::create(ctx, addr.clone()).await.map(|_| addr)
}

#[ockam_node_test_attribute::node_test]
async fn static_simple_pipe(ctx: &mut Context) -> Result<()> {
    receiver(ctx, "pipe-receiver").await?;
    let tx = connect_static(ctx, vec!["pipe-receiver"]).await?;

    let sent_msg = String::from("Hello Ockam!");
    info!("Sending message '{}' through pipe sender {}", sent_msg, tx);
    ctx.send(vec![tx.clone(), "app".into()], sent_msg.clone())
        .await?;

    let msg = ctx.receive().await?;
    info!("App reiceved msg: '{}'", msg);
    assert_eq!(msg, sent_msg);

    ctx.stop().await
}

#[ockam_node_test_attribute::node_test]
async fn static_confirm_pipe(ctx: &mut Context) -> Result<()> {
    receiver_with_behavior(ctx, "pipe-receiver", PipeBehavior::with(ReceiverConfirm)).await?;
    let tx = connect_static_with_behavior(
        ctx,
        vec!["pipe-receiver"],
        PipeBehavior::with(SenderConfirm::new()),
    )
    .await?;

    let sent_msg = String::from("Hello Ockam!");
    info!("Sending message '{}' through pipe sender {}", sent_msg, tx);
    ctx.send(vec![tx.clone(), "app".into()], sent_msg.clone())
        .await?;

    let msg = ctx.receive().await?;
    info!("App reiceved msg: '{}'", msg);
    assert_eq!(msg, sent_msg);

    ctx.stop().await
}

/// Create hook that sends a message when the send timeout has elapsed
struct ConfirmTimeout;

#[async_trait]
impl BehaviorHook for ConfirmTimeout {
    async fn on_internal(
        &mut self,
        _: Address,
        _: Route,
        ctx: &mut Context,
        msg: &InternalCmd,
    ) -> Result<()> {
        match msg {
            InternalCmd::Resend(_) => {
                info!("Sender received timeout for sent message!");
                ctx.send("app", "Shut it down...".to_string()).await
            }
            _ => unreachable!(),
        }
    }

    async fn on_external(
        &mut self,
        _: Address,
        _: Route,
        _: &mut Context,
        _: &PipeMessage,
    ) -> Result<()> {
        Ok(())
    }
}

#[ockam_node_test_attribute::node_test]
async fn fails_static_confirm_pipe(ctx: &mut Context) -> Result<()> {
    receiver(ctx, "pipe-receiver").await?;
    let tx = connect_static_with_behavior(
        ctx,
        vec!["pipe-receiver"],
        PipeBehavior::with(SenderConfirm::new()).add(ConfirmTimeout),
    )
    .await?;

    let sent_msg = String::from("Hello Ockam!");
    info!("Sending message '{}' through pipe sender {}", sent_msg, tx);
    ctx.send(vec![tx.clone(), "app".into()], sent_msg.clone())
        .await?;

    let msg = ctx.receive::<String>().await?;
    info!("App reiceved msg: '{}'", msg);
    assert_eq!(msg, sent_msg);

    let invalid = ctx.receive::<String>().await?;
    warn!("App reiceved msg: '{}'", invalid);
    assert_eq!(invalid, "Shut it down...".to_string());

    ctx.stop().await
}
