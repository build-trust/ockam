// This test checks that an attribute macro #[ockam::node] exists
// and can be used with an async main function

#[ockam::node]
async fn main(context: ockam::Context) {
    context.stop().await.unwrap();
}
