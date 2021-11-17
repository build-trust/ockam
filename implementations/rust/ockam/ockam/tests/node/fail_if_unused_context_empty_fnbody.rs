// This test checks that #[ockam_macros::node] causes a compile time error
// if the function is passed a parameter of type `ockam::Context` but is unused.

#[ockam_macros::node]
async fn main(_ctx: ockam::Context) {}
