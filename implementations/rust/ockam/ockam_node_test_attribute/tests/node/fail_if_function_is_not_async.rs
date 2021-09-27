// This test checks that #[ockam_node_test_attribute::node] causes a compile time error
// if the item it is defined on is not an async function.

#[ockam_node_test_attribute::node]
fn main(mut c: ockam::Context) {
    c.stop().await.unwrap();
}
