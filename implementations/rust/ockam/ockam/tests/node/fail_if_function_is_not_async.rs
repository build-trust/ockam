// This test checks that #[ockam_macros::node] causes a compile time error
// if the item it is defined on is not an async function.

#[ockam_macros::node]
fn main(mut c: ockam::Context) {
    c.stop().await.unwrap();
}
