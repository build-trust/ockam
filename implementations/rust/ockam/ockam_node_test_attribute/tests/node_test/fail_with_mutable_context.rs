#[ockam_node_test_attribute::node_test]
async fn my_test(mut ctx: Context) -> ockam::Result<()> {
    ctx.stop().await.unwrap();
}
