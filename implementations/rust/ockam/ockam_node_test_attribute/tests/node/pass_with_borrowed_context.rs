use ockam::Context;

#[ockam_node_test_attribute::node]
async fn main(ctx: &Context) -> ockam::Result<()> {
    ctx.stop().await
}
