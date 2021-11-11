// This test checks that #[ockam_test_macros::node_test] causes a compile time error
// if the function is passed a parameter of type `ockam_node::Context` but is unused.

#[ockam_test_macros::node_test]
async fn my_test(_ctx: &mut ockam_node::Context) -> ockam_core::Result<()> {
    // _ctx.stop().unwrap();
    let _x = 42 as u8;
    Ok(())
}
