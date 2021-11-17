#[ockam_macros::node]
async fn main(mut ctx: ockam::Context) -> ockam_core::Result<()> {
    ctx.stop().await
}
