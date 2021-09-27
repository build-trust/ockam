use ockam_core::traits::AsyncClone;
use ockam_key_exchange_core::{KeyExchanger, NewKeyExchanger};
use ockam_vault_core::SymmetricVault;

/// Vault with XX required functionality
pub trait SecureChannelVault: SymmetricVault + AsyncClone + Clone + Send + 'static {}

impl<D> SecureChannelVault for D where D: SymmetricVault + AsyncClone + Clone + Send + 'static {}

/// Vault with XX required functionality
pub trait SecureChannelKeyExchanger: KeyExchanger + Send + 'static {}

impl<D> SecureChannelKeyExchanger for D where D: KeyExchanger + Send + 'static {}

/// Vault with XX required functionality
pub trait SecureChannelNewKeyExchanger: NewKeyExchanger + Send + 'static {}

impl<D> SecureChannelNewKeyExchanger for D where D: NewKeyExchanger + Send + 'static {}
