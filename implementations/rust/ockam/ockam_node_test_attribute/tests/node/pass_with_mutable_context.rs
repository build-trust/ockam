#[ockam_node_test_attribute::node]
async fn main(mut ctx: Context) {
    ctx.stop().await.unwrap();
}
