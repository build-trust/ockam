// This test checks that an attribute macro #[ockam::node]
// exists and can be used with any async function.

#[ockam::node]
async fn hundred_times(n: u16) -> u16 {
    n * 100
}

fn main() {
    let r = hundred_times(100);
    assert!(r == 10000);
}
