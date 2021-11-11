// Test case to verify that only one argument is passed.
//
#[ockam_macros::node]
async fn main(mut c: ockam::Context, _x: u64) {
    c.stop().await.unwrap();
}
