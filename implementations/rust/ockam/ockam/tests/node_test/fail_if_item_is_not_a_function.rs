// This test checks that #[ockam_test_macros::node_test] causes a compile time error
// if the item it is defined on is not a function.

#[ockam_test_macros::node_test]
struct A {}
