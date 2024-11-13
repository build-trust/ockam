#[ockam::node]
async fn main(c: ockam::Context, _x: u64) -> ockam_core::Result<()> {
    c.shutdown_node().await
}
