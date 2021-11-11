use crate::{
    pipe::*,
    protocols::pipe::{internal::InternalCmd, PipeMessage},
    Context,
};
use ockam_core::{async_trait, Address, Result, Route};

use super::behavior::ReceiverOrdering;

#[ockam_test_macros::node_test]
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

#[ockam_test_macros::node_test]
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

#[ockam_test_macros::node_test]
async fn fails_static_confirm_pipe(ctx: &mut Context) -> Result<()> {
    receiver_with_behavior(ctx, "pipe-receiver", DropDelivery).await?;
    let tx = connect_static_with_behavior(
        ctx,
        vec!["pipe-receiver"],
        PipeBehavior::with(SenderConfirm::new()).attach(ConfirmTimeout),
    )
    .await?;

    let sent_msg = String::from("Hello Ockam!");
    info!("Sending message '{}' through pipe sender {}", sent_msg, tx);
    ctx.send(vec![tx.clone(), "app".into()], sent_msg.clone())
        .await?;

    let invalid = ctx.receive::<String>().await?;
    warn!("App reiceved msg: '{}'", invalid);
    assert_eq!(invalid, "Shut it down...".to_string());

    ctx.stop_now().await
}

/// A simple test to ensure static ordering pipes can deliver messages
#[ockam_test_macros::node_test]
async fn static_ordering_pipe(ctx: &mut Context) -> Result<()> {
    receiver_with_behavior(ctx, "pipe-receiver", ReceiverOrdering::new()).await?;
    let tx = connect_static(ctx, "pipe-receiver").await?;

    let sent_msg1 = String::from("Message number one");
    info!("Sending message '{}' through pipe sender {}", sent_msg1, tx);
    ctx.send(vec![tx.clone(), "app".into()], sent_msg1.clone())
        .await?;

    let sent_msg2 = String::from("Message number two");
    info!("Sending message '{}' through pipe sender {}", sent_msg2, tx);
    ctx.send(vec![tx.clone(), "app".into()], sent_msg2.clone())
        .await?;

    let msg1 = ctx.receive().await?;
    info!("App reiceved msg: '{}'", msg1);
    assert_eq!(msg1, sent_msg1);

    let msg2 = ctx.receive().await?;
    info!("App reiceved msg: '{}'", msg2);
    assert_eq!(msg2, sent_msg2);

    ctx.stop().await
}

#[ockam_test_macros::node_test]
async fn simple_pipe_handshake(ctx: &mut Context) -> Result<()> {
    // Create a pipe spawn listener and connect to it via a dynamic sender
    let listener = listen(ctx).await.unwrap();
    let tx = connect_dynamic(ctx, listener.into()).await.unwrap();

    let msg_sent = String::from("Message for my best friend");
    info!("Sending message '{}' through pipe sender {}", msg_sent, tx);
    ctx.send(vec![tx, "app".into()], msg_sent.clone()).await?;

    let msg = ctx.receive().await?;
    info!("App received msg: '{}'", msg);
    assert_eq!(msg, msg_sent);

    ctx.stop().await
}
