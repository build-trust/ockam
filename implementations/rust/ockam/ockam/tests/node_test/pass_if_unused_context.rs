#[ockam::test]
async fn my_test(_ctx: &mut ockam_node::Context) -> ockam_core::Result<()> {
    let _x = 42 as u8;
    Ok(())
}

fn main() {}
