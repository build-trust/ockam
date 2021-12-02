// This test checks that #[ockam::node] causes a compile time error
// if the function is passed a param that is not of type `ockam::Context`

#[ockam::node]
async fn main(ctx: std::string::String) -> Result<()> {
    Ok(())
}
