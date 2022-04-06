#[ockam::node(crate = "ockam_node")]
async fn main(mut ctx: ockam_node::Context) -> ockam_core::Result<()> {
    ctx.stop().await
}
