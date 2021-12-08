use ockam_key_exchange_core::{KeyExchanger, NewKeyExchanger};
use ockam_vault_core::SymmetricVault;

/// Vault with XX required functionality
pub trait SecureChannelVault: SymmetricVault + Send + Sync + 'static {}

impl<D> SecureChannelVault for D where D: SymmetricVault + Send + Sync + ?Sized + 'static {}

/// KeyExchanger with extra constraints
pub trait SecureChannelKeyExchanger: KeyExchanger + Send + Sync + 'static {}

impl<D> SecureChannelKeyExchanger for D where D: KeyExchanger + Send + Sync + 'static {}

/// NewKeyExchanger with extra constraints
pub trait SecureChannelNewKeyExchanger: NewKeyExchanger + Send + Sync + 'static {}

impl<D> SecureChannelNewKeyExchanger for D where D: NewKeyExchanger + Send + Sync + 'static {}
