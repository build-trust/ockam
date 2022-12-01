//! Ockam channel tests
use ockam::{
    channel::*,
    pipe::{ReceiverConfirm, ReceiverOrdering, SenderConfirm},
    Context,
};
use ockam_core::{route, AllowAll, Result};
use std::sync::Arc;
use tracing::info;

#[ockam::test]
async fn simple_channel(ctx: &mut Context) -> Result<()> {
    let builder = ChannelBuilder::new(ctx).await?;

    // Create a channel listener
    builder
        .create_channel_listener("my-channel-listener")
        .await?;

    // Create a channel via the listener.  We re-use the
    // ChannelBuilder here but could also use a new one
    let ch = builder.connect(route!["my-channel-listener"]).await?;

    // Send a message through the channel
    let msg = "Hello through the channel!".to_string();
    let mut child_ctx = ctx
        .new_detached_with_access_control("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx.send(ch.tx().append("child"), msg.clone()).await?;

    // Then wait for the message through the channel
    let recv = child_ctx.receive().await?;
    info!("Received message '{}' through channel", recv);
    assert_eq!(recv, msg);

    ctx.stop().await
}

#[ockam::test]
async fn reliable_channel(ctx: &mut Context) -> Result<()> {
    let builder = ChannelBuilder::new(ctx)
        .await?
        .attach_rx_behavior(ReceiverConfirm)
        .attach_rx_behavior(ReceiverOrdering::new())
        .attach_tx_behavior(SenderConfirm::new());

    // Create a channel listener
    builder
        .create_channel_listener("my-channel-listener")
        .await?;

    // Create a channel via the listener.  We re-use the
    // ChannelBuilder here but could also use a new one
    let ch = builder.connect("my-channel-listener").await?;

    // Send a message through the channel
    let msg = "Hello through the channel!".to_string();
    let mut child_ctx = ctx
        .new_detached_with_access_control("child", Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    child_ctx.send(ch.tx().append("child"), msg.clone()).await?;

    // Then wait for the message through the channel
    let recv = child_ctx.receive().await?;
    info!("Received message '{}' through channel", recv);
    assert_eq!(recv, msg);

    ctx.stop().await
}
