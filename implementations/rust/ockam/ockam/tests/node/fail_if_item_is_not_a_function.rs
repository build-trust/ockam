// This test checks that #[ockam::node] causes a compile time error
// if the item it is defined on is not a function.

#[ockam::node]
struct A {}
