use crate::pipe::*;
use crate::protocols::pipe::{internal::InternalCmd, PipeMessage};
use ockam_core::{async_trait, Address, Result, Route};
use ockam_node::Context;

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

struct DelayDelivery;

#[async_trait]
impl BehaviorHook for DelayDelivery {
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
        ctx: &mut Context,
        _: &PipeMessage,
    ) -> Result<()> {
        ctx.sleep(core::time::Duration::from_secs(10)).await;
        Ok(())
    }
}

#[ockam_node_test_attribute::node_test]
async fn fails_static_confirm_pipe(ctx: &mut Context) -> Result<()> {
    receiver_with_behavior(ctx, "pipe-receiver", DelayDelivery).await?;
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

    let invalid = ctx.receive::<String>().await?;
    warn!("App reiceved msg: '{}'", invalid);
    assert_eq!(invalid, "Shut it down...".to_string());

    ctx.stop_now().await
}
