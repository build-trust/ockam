#![deny(unused_imports)]

use ockam::Context;

#[ockam::node]
async fn main(c: Context) -> ockam_core::Result<()> {
    c.shutdown_node().await
}
