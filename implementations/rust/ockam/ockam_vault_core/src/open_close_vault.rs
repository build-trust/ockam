use crate::vault::Vault;
use ockam_core::Error;

pub trait OpenCloseVault: Sized {
    fn open(&mut self) -> Result<Vault<'_, Self>, Error>;

    fn close(&mut self);
}
