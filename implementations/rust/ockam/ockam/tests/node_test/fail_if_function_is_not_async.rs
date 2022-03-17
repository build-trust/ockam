#[ockam::test]
fn my_test(c: &mut ockam_node::Context) -> ockam_core::Result<()> {
    c.stop().await.unwrap();
}

fn main() {}
