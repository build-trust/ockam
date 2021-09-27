#[ockam_node_test_attribute::node]
async fn foo(ctx: &Context) {
    ctx.stop().await.unwrap();
}
