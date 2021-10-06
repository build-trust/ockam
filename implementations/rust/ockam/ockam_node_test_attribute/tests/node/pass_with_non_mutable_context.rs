#[ockam_node_test_attribute::node]
async fn main(ctx: Context) {
    ctx.stop().await.unwrap();
}
