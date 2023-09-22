#[ockam::node]
fn foo(c: ockam::Context) {
    c.stop().await.unwrap();
}
