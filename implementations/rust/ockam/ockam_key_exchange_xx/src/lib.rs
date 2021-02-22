use ockam_vault_core::{AsymmetricVault, Hasher, SecretVault, SymmetricVault};

mod error;
pub use error::*;

/// The number of bytes in a SHA256 digest
pub const SHA256_SIZE: usize = 32;
/// The number of bytes in AES-GCM tag
pub const AES_GCM_TAGSIZE: usize = 16;

/// Vault with XX required functionality
pub trait XXVault: SecretVault + Hasher + AsymmetricVault + SymmetricVault + Send {}

impl<D> XXVault for D where D: SecretVault + Hasher + AsymmetricVault + SymmetricVault + Send {}

mod initiator;
mod state;
pub use initiator::*;
mod responder;
pub use responder::*;
mod new_key_exchanger;
pub use new_key_exchanger::*;

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_key_exchange_core::{KeyExchanger, NewKeyExchanger};
    use ockam_vault::SoftwareVault;
    use std::sync::{Arc, Mutex};

    #[allow(non_snake_case)]
    #[test]
    fn full_flow__correct_credentials__keys_should_match() {
        let vault_initiator = Arc::new(Mutex::new(SoftwareVault::default()));
        let vault_responder = Arc::new(Mutex::new(SoftwareVault::default()));
        let key_exchanger =
            XXNewKeyExchanger::new(vault_initiator.clone(), vault_responder.clone());

        let mut initiator = key_exchanger.initiator();
        let mut responder = key_exchanger.responder();

        let m1 = initiator.process(&[]).unwrap();
        let _ = responder.process(&m1).unwrap();
        let m2 = responder.process(&[]).unwrap();
        let _ = initiator.process(&m2).unwrap();
        let m3 = initiator.process(&[]).unwrap();
        let _ = responder.process(&m3).unwrap();

        let initiator = Box::new(initiator);
        let initiator = initiator.finalize().unwrap();
        let responder = Box::new(responder);
        let responder = responder.finalize().unwrap();

        let mut vault_in = vault_initiator.lock().unwrap();
        let mut vault_re = vault_responder.lock().unwrap();

        assert_eq!(initiator.h(), responder.h());

        let s1 = vault_in.secret_export(&initiator.encrypt_key()).unwrap();
        let s2 = vault_re.secret_export(&responder.decrypt_key()).unwrap();

        assert_eq!(s1, s2);

        let s1 = vault_in.secret_export(&initiator.decrypt_key()).unwrap();
        let s2 = vault_re.secret_export(&responder.encrypt_key()).unwrap();

        assert_eq!(s1, s2);
    }
}
