use ockam_key_exchange_core::{Cipher, KeyExchanger, NewKeyExchanger};

/// Cipher used with Secure Channel
pub trait SecureChannelCipher: Cipher + Send + Sync + 'static {}

impl<D> SecureChannelCipher for D where D: Cipher + Send + Sync + 'static {}

/// KeyExchanger with extra constraints
pub trait SecureChannelKeyExchanger: KeyExchanger + Send + Sync + 'static {}

impl<D> SecureChannelKeyExchanger for D where D: KeyExchanger + Send + Sync + 'static {}

/// NewKeyExchanger with extra constraints
pub trait SecureChannelNewKeyExchanger: NewKeyExchanger + Send + Sync + 'static {}

impl<D> SecureChannelNewKeyExchanger for D where D: NewKeyExchanger + Send + Sync + 'static {}
