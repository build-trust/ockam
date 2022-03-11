// This test checks that #[ockam::node] causes a compile time error
// if the function is passed a `self` param (thus making it a
// `Receiver` function.

#[ockam::node]
async fn main(self) {
    let a = 1;
}
