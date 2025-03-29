// This file is intended to be run while developing macros.
// To expand this file run `cargo expand --test macro_expand_playground`.

// Since this is just a convenience file to make macros development
// easier, it's not recommended to commit modifications of this file.

#[ockam::test(crate = "ockam", timeout = 100)]
#[ignore]
async fn my_test(ctx: &mut ockam::Context) -> ockam::Result<()> {
    ctx.shutdown_node().await
}
