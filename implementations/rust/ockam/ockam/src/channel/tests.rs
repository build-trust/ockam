//! Ockam channel tests
use crate::{
    channel::*,
    pipe::{ReceiverConfirm, ReceiverOrdering, SenderConfirm},
    Context,
};
use ockam_core::Result;

#[crate::test]
async fn simple_channel(ctx: &mut Context) -> Result<()> {
    let builder = ChannelBuilder::new(ctx).await?;

    // Create a channel listener
    builder
        .create_channel_listener("my-channel-listener")
        .await?;

    // Create a channel via the listener.  We re-use the
    // ChannelBuilder here but could also use a new one
    let ch = builder.connect(vec!["my-channel-listener"]).await?;

    // Send a message through the channel
    let msg = "Hello through the channel!".to_string();
    ctx.send(ch.tx().append("app"), msg.clone()).await?;

    // Then wait for the message through the channel
    let recv = ctx.receive().await?;
    info!("Received message '{}' through channel", recv);
    assert_eq!(recv, msg);

    ctx.stop().await
}

#[crate::test]
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
    let ch = builder.connect(vec!["my-channel-listener"]).await?;

    // Send a message through the channel
    let msg = "Hello through the channel!".to_string();
    ctx.send(ch.tx().append("app"), msg.clone()).await?;

    // Then wait for the message through the channel
    let recv = ctx.receive().await?;
    info!("Received message '{}' through channel", recv);
    assert_eq!(recv, msg);

    ctx.stop().await
}
