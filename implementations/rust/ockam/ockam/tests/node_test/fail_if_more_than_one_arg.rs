#[ockam::test]
async fn my_test(mut c: ockam_node::Context, _x: u64) -> ockam_core::Result<()> {
    c.shutdown_node().await.unwrap();
}

fn main() {}
