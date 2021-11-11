#[ockam_macros::node]
async fn main(ctx: &mut ockam::Context) -> ockam_core::Result<()> {
    ctx.stop().await
}
