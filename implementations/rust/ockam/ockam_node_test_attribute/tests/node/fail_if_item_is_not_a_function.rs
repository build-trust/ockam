// This test checks that #[ockam_node_test_attribute::node] causes a compile time error
// if the item it is defined on is not a function.

#[ockam_node_test_attribute::node]
struct A {}
