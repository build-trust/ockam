use ockam_core::AsyncTryClone;

#[derive(AsyncTryClone)]
pub struct Tmp {
    a: u32,
}

#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam")]
pub struct Tmp1<T> {
    a: u32,
    b: Vec<T>,
}

#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct Tmp2<T> {
    a: u32,
    b: T,
}

fn assert_impl<T: AsyncTryClone>() {}
fn main() {
    assert_impl::<String>();
    assert_impl::<Tmp>();
    assert_impl::<Tmp1<usize>>();
    assert_impl::<Tmp2<Tmp>>();
    assert_impl::<Tmp2<Tmp1<String>>>();
}
