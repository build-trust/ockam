#[ockam::node]
async fn foo(c: ockam::Context) {
    c.stop().unwrap();
}
