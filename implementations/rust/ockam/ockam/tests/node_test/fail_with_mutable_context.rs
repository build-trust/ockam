#[ockam::test]
async fn my_test(mut ctx: Context) -> ockam_core::Result<()> {
    ctx.shutdown_node().await.unwrap();
}

fn main() {}
