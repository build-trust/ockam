mod examples;
use examples::node;
use examples::routing;
use examples::worker;
use hello_ockam::log_collector::LogCollector;
use ockam_core::{AsyncTryClone, Result};
use ockam_node::Context;

#[ockam_macros::test(no_logging)]
async fn test_node(ctx: &mut Context) -> Result<()> {
    let collector = LogCollector::setup();

    node::main_function(ctx.async_try_clone().await?).await?;

    assert!(collector.contains("Goodbye"));
    Ok(())
}

#[ockam_macros::test(no_logging)]
async fn test_worker(ctx: &mut Context) -> Result<()> {
    let collector = LogCollector::setup();

    worker::main_function(ctx.async_try_clone().await?).await?;

    assert!(collector.contains("Goodbye"));
    Ok(())
}

#[ockam_macros::test(no_logging)]
async fn test_routing(ctx: &mut Context) -> Result<()> {
    let collector = LogCollector::setup();

    routing::main_function(ctx.async_try_clone().await?).await?;

    assert!(collector.contains("Goodbye"));
    Ok(())
}
