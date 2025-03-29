use ockam_core::TryClone;

#[derive(TryClone)]
pub struct Tmp {
    a: u32,
}

#[derive(TryClone)]
#[try_clone(crate = "ockam")]
pub struct Tmp1<T> {
    a: u32,
    b: Vec<T>,
}

#[derive(TryClone)]
#[try_clone(crate = "ockam_core")]
pub struct Tmp2<T> {
    a: u32,
    b: T,
}

fn assert_impl<T: TryClone>() {}
fn main() {
    assert_impl::<String>();
    assert_impl::<Tmp>();
    assert_impl::<Tmp1<usize>>();
    assert_impl::<Tmp2<Tmp>>();
    assert_impl::<Tmp2<Tmp1<String>>>();
}
