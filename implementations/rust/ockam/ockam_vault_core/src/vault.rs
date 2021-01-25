use crate::open_close_vault::OpenCloseVault;

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
