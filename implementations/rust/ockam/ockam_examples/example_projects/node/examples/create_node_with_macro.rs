#[ockam::node]
async fn main(context: ockam::Context) -> Result<()> {
    context.stop().await
}
