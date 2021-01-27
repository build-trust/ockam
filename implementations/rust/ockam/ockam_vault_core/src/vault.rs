use crate::open_close_vault::OpenCloseVault;
use std::ops::{Deref, DerefMut};

pub struct Vault<'a, T>
where
    T: OpenCloseVault,
{
    pub(crate) inner: &'a mut T,
}

impl<'a, T> Vault<'a, T>
where
    T: OpenCloseVault,
{
    pub fn new(inner: &'a mut T) -> Self {
        Vault { inner }
    }
}

impl<T> Drop for Vault<'_, T>
where
    T: OpenCloseVault,
{
    #[inline]
    fn drop(&mut self) {
        self.inner.close()
    }
}

impl<'a, T> Deref for Vault<'a, T>
where
    T: OpenCloseVault,
{
    type Target = T::InnerVault;

    fn deref(&self) -> &T::InnerVault {
        self.inner.get_data()
    }
}

impl<'a, T> DerefMut for Vault<'a, T>
where
    T: OpenCloseVault,
{
    fn deref_mut(&mut self) -> &mut T::InnerVault {
        self.inner.get_data_mut()
    }
}
