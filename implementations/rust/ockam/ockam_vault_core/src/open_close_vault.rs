use crate::vault::Vault;
use ockam_core::Error;

pub trait OpenCloseVault: Sized {
    type InnerVault;

    fn get_data_mut(&mut self) -> &mut Self::InnerVault;

    fn get_data(&self) -> &Self::InnerVault;

    fn open(&mut self) -> Result<Vault<'_, Self>, Error>;

    fn close(&mut self);
}
