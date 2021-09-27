#[ockam_node_test_attribute::node]
async fn foo(mut ctx: Context) {
    ctx.stop().await.unwrap();
}
