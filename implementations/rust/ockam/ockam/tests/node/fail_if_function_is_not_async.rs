// This test checks that #[ockam::node] causes a compile time error
// if the item it is defined on is not an async function.

#[ockam::node]
fn main(mut c: ockam::Context) {
    c.stop().await.unwrap();
}
