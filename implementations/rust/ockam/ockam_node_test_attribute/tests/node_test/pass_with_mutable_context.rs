#[ockam_node_test_attribute::node_test]
async fn my_test(mut ctx: Context) {
    ctx.stop().await.unwrap();
}
