#[ockam_node_test_attribute::node]
async fn main(mut ctx: ockam::Context) {
    ctx.stop().await.unwrap();
}
