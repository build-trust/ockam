use ockam::compat::tokio::time::sleep;
use ockam_core::Result;
use ockam_node::Context;
use std::time::Duration;

#[ockam_macros::test]
async fn ok_if_return_ok(ctx: &mut Context) -> Result<()> {
    ctx.stop().await
}

#[ockam_macros::test]
#[should_panic]
async fn fail_if_return_err(_ctx: &mut Context) -> Result<()> {
    Err(ockam_core::Error::new_without_cause(
        ockam_core::errcode::Origin::Node,
        ockam_core::errcode::Kind::Invalid,
    ))
}

#[ockam_macros::test]
#[should_panic]
async fn fail_if_test_panics(_ctx: &mut Context) -> Result<()> {
    panic!("Expected panic called");
}

#[ockam_macros::test(timeout = 0)]
#[should_panic]
async fn fail_if_test_times_out(_ctx: &mut Context) -> Result<()> {
    sleep(Duration::from_millis(100)).await;
    Ok(())
}
