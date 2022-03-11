#[ockam::node]
fn foo(mut c: ockam::Context) {
    c.stop().await.unwrap();
}
