use ockam_core::AsyncTryClone;

#[derive(AsyncTryClone)]
#[async_try_clone(crate = "my_crate")]
pub struct Tmp {
    a: u32,
}

fn main() {}
