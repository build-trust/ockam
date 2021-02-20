// Test case to verify that only one argument is passed.
//
#[ockam::node]
async fn foo(c: ockam::Context, _x: u64) {
    c.stop().unwrap();
}
