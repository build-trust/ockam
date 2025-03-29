use ockam_core::TryClone;

#[derive(TryClone)]
#[try_clone(crate = "my_crate")]
pub struct Tmp {
    a: u32,
}

fn main() {}
