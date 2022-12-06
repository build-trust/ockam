use ockam::{
    pipe::*,
    protocols::pipe::{internal::InternalCmd, PipeMessage},
    Context,
};
use ockam_core::{async_trait, route, Address, AllowAll, Result, Route};
use std::sync::Arc;
use tracing::{info, warn};

#[ockam::test]
async fn static_simple_pipe(ctx: &mut Context) -> Result<()> {
    receiver(ctx, "pipe-receiver").await?;
    let tx = connect_static(ctx, "pipe-receiver").await?;

    let sent_msg = String::from("Hello Ockam!");
    info!("Sending message '{}' through pipe sender {}", sent_msg, tx);
    let mut child_ctx = ctx
        .new_detached("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx
        .send(route![tx.clone(), "child"], sent_msg.clone())
        .await?;

    let msg = child_ctx.receive().await?;
    info!("App received msg: '{}'", msg);
    assert_eq!(msg, sent_msg);

    ctx.stop().await
}

#[ockam::test]
async fn static_confirm_pipe(ctx: &mut Context) -> Result<()> {
    receiver_with_behavior(ctx, "pipe-receiver", PipeBehavior::with(ReceiverConfirm)).await?;
    let tx = connect_static_with_behavior(
        ctx,
        "pipe-receiver",
        PipeBehavior::with(SenderConfirm::new()),
    )
    .await?;

    let sent_msg = String::from("Hello Ockam!");
    info!("Sending message '{}' through pipe sender {}", sent_msg, tx);
    let mut child_ctx = ctx
        .new_detached("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx
        .send(route![tx.clone(), "child"], sent_msg.clone())
        .await?;

    let msg = child_ctx.receive().await?;
    info!("App received msg: '{}'", msg);
    assert_eq!(msg, sent_msg);

    ctx.stop().await
}

/// Create hook that sends a message when the send timeout has elapsed
#[derive(Clone)]
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
                ctx.send("child", "Shut it down...".to_string()).await
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
    ) -> Result<PipeModifier> {
        Ok(PipeModifier::None)
    }
}

#[derive(Clone)]
struct DropDelivery;

#[async_trait]
impl BehaviorHook for DropDelivery {
    async fn on_internal(
        &mut self,
        _: Address,
        _: Route,
        _: &mut Context,
        _: &InternalCmd,
    ) -> Result<()> {
        Ok(())
    }

    async fn on_external(
        &mut self,
        _: Address,
        _: Route,
        _: &mut Context,
        _: &PipeMessage,
    ) -> Result<PipeModifier> {
        // Simply instruct the receiver to drop the message
        Ok(PipeModifier::Drop)
    }
}

#[ockam::test]
async fn fails_static_confirm_pipe(ctx: &mut Context) -> Result<()> {
    receiver_with_behavior(ctx, "pipe-receiver", DropDelivery).await?;
    let tx = connect_static_with_behavior(
        ctx,
        "pipe-receiver",
        PipeBehavior::with(SenderConfirm::new()).attach(ConfirmTimeout),
    )
    .await?;

    let sent_msg = String::from("Hello Ockam!");
    info!("Sending message '{}' through pipe sender {}", sent_msg, tx);
    let mut child_ctx = ctx
        .new_detached("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx
        .send(route![tx.clone(), "child"], sent_msg.clone())
        .await?;

    let invalid = child_ctx.receive::<String>().await?;
    warn!("App received msg: '{}'", invalid);
    assert_eq!(invalid, "Shut it down...".to_string());

    ctx.stop().await
}

/// A simple test to ensure static ordering pipes can deliver messages
#[ockam::test]
async fn static_ordering_pipe(ctx: &mut Context) -> Result<()> {
    receiver_with_behavior(ctx, "pipe-receiver", ReceiverOrdering::new()).await?;
    let tx = connect_static(ctx, "pipe-receiver").await?;

    let sent_msg1 = String::from("Message number one");
    info!("Sending message '{}' through pipe sender {}", sent_msg1, tx);
    let mut child_ctx = ctx
        .new_detached("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx
        .send(route![tx.clone(), "child"], sent_msg1.clone())
        .await?;

    let sent_msg2 = String::from("Message number two");
    info!("Sending message '{}' through pipe sender {}", sent_msg2, tx);
    child_ctx
        .send(route![tx.clone(), "child"], sent_msg2.clone())
        .await?;

    let msg1 = child_ctx.receive().await?;
    info!("App received msg: '{}'", msg1);
    assert_eq!(msg1, sent_msg1);

    let msg2 = child_ctx.receive().await?;
    info!("App received msg: '{}'", msg2);
    assert_eq!(msg2, sent_msg2);

    ctx.stop().await
}

/// A test for a pipe that enforces ordering _and_ sends confirm messages
#[ockam::test]
async fn static_confirm_ordering_pipe(ctx: &mut Context) -> Result<()> {
    receiver_with_behavior(
        ctx,
        "pipe-receiver",
        PipeBehavior::with(ReceiverConfirm).attach(ReceiverOrdering::new()),
    )
    .await?;

    let tx = connect_static_with_behavior(
        ctx,
        "pipe-receiver",
        PipeBehavior::with(SenderConfirm::new()),
    )
    .await?;

    let sent_msg1 = String::from("Message number one");
    info!("Sending message '{}' through pipe sender {}", sent_msg1, tx);
    let mut child_ctx = ctx
        .new_detached("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx
        .send(route![tx.clone(), "child"], sent_msg1.clone())
        .await?;

    let sent_msg2 = String::from("Message number two");
    info!("Sending message '{}' through pipe sender {}", sent_msg2, tx);
    child_ctx
        .send(route![tx.clone(), "child"], sent_msg2.clone())
        .await?;

    let msg1 = child_ctx.receive().await?;
    info!("App received msg: '{}'", msg1);
    assert_eq!(msg1, sent_msg1);

    let msg2 = child_ctx.receive().await?;
    info!("App received msg: '{}'", msg2);
    assert_eq!(msg2, sent_msg2);

    ctx.stop().await
}

/// A test for a pipe that enforces ordering _and_ sends confirm
/// messages but with a flipped behaviour order on the receiver end
#[ockam::test]
async fn static_confirm_ordering_pipe_reversed(ctx: &mut Context) -> Result<()> {
    receiver_with_behavior(
        ctx,
        "pipe-receiver",
        PipeBehavior::with(ReceiverOrdering::new()).attach(ReceiverConfirm),
    )
    .await?;

    let tx = connect_static_with_behavior(
        ctx,
        "pipe-receiver",
        PipeBehavior::with(SenderConfirm::new()),
    )
    .await?;

    let sent_msg1 = String::from("Message number one");
    info!("Sending message '{}' through pipe sender {}", sent_msg1, tx);
    let mut child_ctx = ctx
        .new_detached("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx
        .send(route![tx.clone(), "child"], sent_msg1.clone())
        .await?;

    let sent_msg2 = String::from("Message number two");
    info!("Sending message '{}' through pipe sender {}", sent_msg2, tx);
    child_ctx
        .send(route![tx.clone(), "child"], sent_msg2.clone())
        .await?;

    let msg1 = child_ctx.receive().await?;
    info!("App received msg: '{}'", msg1);
    assert_eq!(msg1, sent_msg1);

    let msg2 = child_ctx.receive().await?;
    info!("App received msg: '{}'", msg2);
    assert_eq!(msg2, sent_msg2);

    ctx.stop().await
}

#[ockam::test]
async fn simple_pipe_handshake(ctx: &mut Context) -> Result<()> {
    // Create a pipe spawn listener and connect to it via a dynamic sender
    let listener = listen(ctx).await.unwrap();
    let tx = connect_dynamic(ctx, listener.into()).await.unwrap();

    let msg_sent = String::from("Message for my best friend");
    info!("Sending message '{}' through pipe sender {}", msg_sent, tx);
    let mut child_ctx = ctx
        .new_detached("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx
        .send(route![tx, "child"], msg_sent.clone())
        .await?;

    let msg = child_ctx.receive().await?;
    info!("App received msg: '{}'", msg);
    assert_eq!(msg, msg_sent);

    ctx.stop().await
}

#[ockam::test]
async fn layered_pipe(ctx: &mut Context) -> Result<()> {
    // This test creates a pipe with multiple behaviours via layered
    // workers.
    //
    // /------\      /---------\       /---------\      /---------\      /---------\
    // | app  | ---> | confirm | --->  | ordered | -->  | ordered | -->  | confirm | -->
    // |      |      |  sender |       |  sender |      |receiver |      |receiver |
    // \------/      \--------/        \--------/       \--------/       \--------/
    //

    // First create the ordered pipe pair
    receiver_with_behavior(ctx, "order-receiver", ReceiverOrdering::new()).await?;
    let ord_tx = connect_static(ctx, "order-receiver").await?;

    // Then create the confirm pipe pair
    receiver_with_behavior(ctx, "confirm-receiver", ReceiverOrdering::new()).await?;
    let confirm_tx = connect_static(ctx, route![ord_tx.clone(), "confirm-receiver"]).await?;

    // Then we can send a message through this concoction
    let msg = "Hello through nested pipes!".to_string();
    let mut child_ctx = ctx
        .new_detached("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx
        .send(route![confirm_tx, "child"], msg.clone())
        .await?;

    // Wait for the message to arrive
    let msg_recv = child_ctx.receive().await?;
    info!("App received message: {}", msg_recv);
    assert_eq!(msg_recv, msg);

    ctx.stop().await
}
