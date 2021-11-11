// This test checks that #[ockam_macros::node] causes a compile time error
// if the function is passed a param that is not of type `ockam::Context`
// The param is not fully qualified (ie. using `use` statement).

use std::string::String;

#[ockam_macros::node]
async fn main(ctx: String) -> Result<()> {
    Ok(())
}
