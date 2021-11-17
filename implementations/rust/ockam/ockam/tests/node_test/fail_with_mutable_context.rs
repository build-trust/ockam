#[ockam_test_macros::node_test]
async fn my_test(mut ctx: Context) -> ockam_core::Result<()> {
    ctx.stop().await.unwrap();
}
