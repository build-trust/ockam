// This test checks that #[ockam::test] causes a compile time error
// if the item it is defined on is not a function.

#[ockam::test]
struct A {}
