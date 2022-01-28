use core::marker::PhantomData;

/// Async RwLock
// TODO: Implement
#[derive(Debug)]
pub struct RwLock<T: ?Sized> {
    _phantom_data: PhantomData<T>,
}
