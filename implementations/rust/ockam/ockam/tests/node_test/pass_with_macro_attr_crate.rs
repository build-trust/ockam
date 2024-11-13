#[ockam::test(crate = "ockam")]
async fn my_test(ctx: &mut ockam::Context) -> ockam_core::Result<()> {
    ctx.shutdown_node().await
}

fn main() {}
