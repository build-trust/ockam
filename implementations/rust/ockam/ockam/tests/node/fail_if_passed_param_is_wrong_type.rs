// This test checks that #[ockam_macros::node] causes a compile time error
// if the function is passed a param that is not of type `ockam::Context`

#[ockam_macros::node]
async fn main(ctx: std::string::String) -> Result<()> {
    Ok(())
}
