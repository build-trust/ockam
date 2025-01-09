#![deny(unused_imports)]

use ockam::{self as o};

#[ockam::node]
async fn main(c: o::Context) -> ockam_core::Result<()> {
    c.shutdown_node().await
}
