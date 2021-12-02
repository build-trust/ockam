// This test checks that #[ockam::test] causes a compile time error
// if the function is passed a `self` param (thus making it a
// `Receiver` function.

#[ockam::test]
async fn my_test(self) -> ockam_core::Result<()> {}
