// This test checks that #[ockam_node_test_attribute::node_test] causes a compile time error
// if the function is passed a parameter of type `ockam::Context` but is unused.

#[ockam_node_test_attribute::node_test]
async fn my_test(_ctx: ockam::Context) {
    // _ctx.stop().unwrap();
    let _x = 42 as u8;
}
