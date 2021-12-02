// This test checks that #[ockam::test] causes a compile time error
// if the function is passed a parameter of type `ockam_node::Context` but is unused.

#[ockam::test]
async fn my_test(_ctx: &mut ockam_node::Context) -> ockam_core::Result<()> {
    // _ctx.stop().unwrap();
    let _x = 42 as u8;
    Ok(())
}
