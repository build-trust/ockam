use ockam_core::{async_trait, Error};

#[async_trait]
pub trait PurposeKeysRepository:
{
    fn as_purposekey_reader(&self) -> Arc<>;
    fn as_purposekey_rotater(&self) -> Arc<>;
    fn as_purposekey_deleter(&self) -> Arc<>;
}
