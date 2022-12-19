use ockam::{pipe2::PipeBuilder, Context};
use ockam_core::{compat::string::String, route, Address, AllowAll, Result};
use tracing::info;

#[ockam::test]
async fn very_simple_pipe2(ctx: &mut Context) -> Result<()> {
    info!("Starting the test...");
    let rx_addr = Address::random_local();

    // Start a static receiver
    let rx = PipeBuilder::fixed()
        .receive(rx_addr.clone())
        .build(ctx)
        .await?;
    info!("Created receiver pipe: {}", rx.addr());

    // Connect to a static receiver
    let sender = PipeBuilder::fixed().connect(rx_addr).build(ctx).await?;
    info!("Created sender pipe: {}", sender.addr());

    let mut child_ctx = ctx.new_detached("child", AllowAll, AllowAll).await?;
    let msg = String::from("Hello through the pipe");
    child_ctx
        .send(route![sender.addr(), "child"], msg.clone())
        .await?;

    let msg2 = child_ctx.receive::<String>().await?;
    assert_eq!(msg, *msg2);
    ctx.stop().await
}

#[ockam::test]
async fn handshake_pipe(ctx: &mut Context) -> Result<()> {
    let listener = PipeBuilder::dynamic()
        .receive("my-pipe-listener")
        .build(ctx)
        .await?;

    // Point the sender to the listener which will spawn a receiver
    let sender = PipeBuilder::dynamic()
        .connect(listener.addr())
        .build(ctx)
        .await?;

    let mut child_ctx = ctx.new_detached("child", AllowAll, AllowAll).await?;
    let msg = String::from("Hello through the pipe");
    child_ctx
        .send(route![sender.addr(), "child"], msg.clone())
        .await?;

    let msg2 = child_ctx.receive::<String>().await?;
    assert_eq!(msg, *msg2);

    ctx.stop().await
}

#[ockam::test]
async fn fixed_delivery_pipe(ctx: &mut Context) -> Result<()> {
    let rx_addr = Address::random_local();

    // Start a static receiver
    let rx = PipeBuilder::fixed()
        .receive(rx_addr.clone())
        .delivery_ack()
        .build(ctx)
        .await?;
    info!("Created receiver pipe: {}", rx.addr());

    // Connect to a static receiver
    let sender = PipeBuilder::fixed()
        .connect(rx_addr)
        .delivery_ack()
        .build(ctx)
        .await?;

    info!("Created sender pipe: {}", sender.addr());

    let mut child_ctx = ctx.new_detached("child", AllowAll, AllowAll).await?;
    let msg = String::from("Hello through the pipe");
    child_ctx
        .send(route![sender.addr(), "child"], msg.clone())
        .await?;

    let msg2 = child_ctx.receive::<String>().await?;
    assert_eq!(msg, *msg2);
    ctx.stop().await
}

#[ockam::test]
async fn dynamic_delivery_pipe(ctx: &mut Context) -> Result<()> {
    let listener = PipeBuilder::dynamic()
        .receive("my-pipe-listener")
        .delivery_ack()
        .build(ctx)
        .await?;

    // Point the sender to the listener which will spawn a receiver
    let sender = PipeBuilder::dynamic()
        .connect(listener.addr())
        .delivery_ack()
        .build(ctx)
        .await?;

    let mut child_ctx = ctx.new_detached("child", AllowAll, AllowAll).await?;
    let msg = String::from("Hello through the pipe");
    child_ctx
        .send(route![sender.addr(), "child"], msg.clone())
        .await?;

    let msg2 = child_ctx.receive::<String>().await?;
    assert_eq!(msg, *msg2);

    ctx.stop().await
}

#[ockam::test]
async fn fixed_ordering_pipe(ctx: &mut Context) -> Result<()> {
    let rx_addr = Address::random_local();

    // Start a static receiver
    let rx = PipeBuilder::fixed()
        .receive(rx_addr.clone())
        .enforce_ordering()
        .build(ctx)
        .await?;
    info!("Created receiver pipe: {}", rx.addr());

    // Connect to a static receiver
    let sender = PipeBuilder::fixed()
        .connect(rx_addr)
        .enforce_ordering()
        .build(ctx)
        .await?;

    info!("Created sender pipe: {}", sender.addr());

    let mut child_ctx = ctx.new_detached("child", AllowAll, AllowAll).await?;
    let msg = String::from("Hello through the pipe");
    child_ctx
        .send(route![sender.addr(), "child"], msg.clone())
        .await?;

    let msg2 = child_ctx.receive::<String>().await?;
    assert_eq!(msg, *msg2);
    ctx.stop().await
}

#[ockam::test]
async fn fixed_delivery_and_ordering_pipe(ctx: &mut Context) -> Result<()> {
    let rx_addr = Address::random_local();

    // Start a static receiver
    let rx = PipeBuilder::fixed()
        .receive(rx_addr.clone())
        .delivery_ack()
        .enforce_ordering()
        .build(ctx)
        .await?;
    info!("Created receiver pipe: {}", rx.addr());

    // Connect to a static receiver
    let sender = PipeBuilder::fixed()
        .connect(rx_addr)
        .delivery_ack()
        .enforce_ordering()
        .build(ctx)
        .await?;

    info!("Created sender pipe: {}", sender.addr());

    let mut child_ctx = ctx.new_detached("child", AllowAll, AllowAll).await?;
    let msg = String::from("Hello through the pipe");
    child_ctx
        .send(route![sender.addr(), "child"], msg.clone())
        .await?;

    let msg2 = child_ctx.receive::<String>().await?;
    assert_eq!(msg, *msg2);
    ctx.stop().await
}

#[ockam::test]
async fn dynamic_delivery_and_ordering_pipe(ctx: &mut Context) -> Result<()> {
    let listener = PipeBuilder::dynamic()
        .receive("my-pipe-listener")
        .delivery_ack()
        .enforce_ordering()
        .build(ctx)
        .await?;

    // Point the sender to the listener which will spawn a receiver
    let sender = PipeBuilder::dynamic()
        .connect(listener.addr())
        .delivery_ack()
        .enforce_ordering()
        .build(ctx)
        .await?;

    let mut child_ctx = ctx.new_detached("child", AllowAll, AllowAll).await?;
    let msg = String::from("Hello through the pipe");
    child_ctx
        .send(route![sender.addr(), "child"], msg.clone())
        .await?;

    let msg2 = child_ctx.receive::<String>().await?;
    assert_eq!(msg, *msg2);

    ctx.stop().await
}
