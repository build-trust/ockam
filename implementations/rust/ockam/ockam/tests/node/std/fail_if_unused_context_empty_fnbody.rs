// This test checks that #[ockam::node] causes a compile time error
// if the function is passed a parameter of type `ockam::Context` but is unused.

#[ockam::node]
async fn main(_ctx: ockam::Context) {}
