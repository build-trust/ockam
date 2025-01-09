#[ockam::test]
async fn my_test(ctx: Context) -> ockam_core::Result<()> {
    ctx.shutdown_node().await.unwrap();
}

fn main() {}
