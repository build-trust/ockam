// This test checks that #[ockam_macros::node] causes a compile time error
// if the item it is defined on is not a function.

#[ockam_macros::node]
struct A {}
