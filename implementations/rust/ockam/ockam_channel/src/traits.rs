use ockam_core::key_exchange::{KeyExchanger, NewKeyExchanger};
use ockam_core::AsyncTryClone;
use ockam_vault_core::SymmetricVault;

/// Vault with XX required functionality
pub trait SecureChannelVault:
    SymmetricVault + XXVault + AsyncTryClone + Send + Sync + 'static
{
}

impl<D> SecureChannelVault for D where
    D: SymmetricVault + XXVault + AsyncTryClone + Send + Sync + 'static
{
}

/// KeyExchanger with extra constraints
pub trait SecureChannelKeyExchanger: KeyExchanger + Send + Sync + 'static {}

impl<D> SecureChannelKeyExchanger for D where D: KeyExchanger + Send + Sync + 'static {}

/// NewKeyExchanger with extra constraints
pub trait SecureChannelNewKeyExchanger: NewKeyExchanger + Send + Sync + 'static {}

impl<D> SecureChannelNewKeyExchanger for D where D: NewKeyExchanger + Send + Sync + 'static {}
