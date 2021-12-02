use ockam_core::AsyncTryClone;

#[derive(AsyncTryClone)]
pub struct Tmp1 {
    a: u32,
}

#[derive(AsyncTryClone)]
pub struct Tmp<T> {
    a: u32,
    b: Vec<T>,
}

#[derive(AsyncTryClone)]
pub struct Tmp2<T> {
    a: u32,
    b: T,
}

fn assert_impl<T: AsyncTryClone>() {}
fn main() {
    assert_impl::<String>();
    assert_impl::<Tmp1>();
    assert_impl::<Tmp<usize>>();
    assert_impl::<Tmp2<Tmp1>>();
    assert_impl::<Tmp2<Tmp<String>>>();
}
