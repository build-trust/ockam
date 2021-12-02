// This test checks that #[ockam::test] causes a compile time error
// if the item it is defined on is not an async function.

#[ockam::test]
fn my_test(c: &mut ockam_node::Context) -> ockam_core::Result<()> {
    c.stop().await.unwrap();
}
