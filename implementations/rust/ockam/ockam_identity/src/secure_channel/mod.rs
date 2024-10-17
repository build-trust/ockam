/// Access control data for workers
pub mod access_control;
mod addresses;
mod api;
mod decryptor;
mod encryptor;
mod encryptor_worker;
pub(crate) mod handshake;
mod key_tracker;
mod listener;
mod message;
mod nonce;
mod nonce_tracker;
mod options;
mod registry;
mod role;

/// List of trust policies to setup ABAC controls
pub mod trust_policy;

pub use access_control::*;
pub(crate) use addresses::*;
pub use api::*;
pub(crate) use decryptor::*;
pub(crate) use encryptor_worker::*;
pub(crate) use handshake::*;
pub(crate) use listener::*;
pub use message::*;
pub use nonce::*;
pub use options::*;
pub use registry::*;
pub(crate) use role::*;
pub use trust_policy::*;

#[cfg(test)]
mod tests {
    use crate::secure_channel::{decryptor::Decryptor, encryptor::Encryptor};
    use ockam_core::compat::rand::RngCore;
    use ockam_core::Result;
    use ockam_vault::{SoftwareVaultForSecureChannels, VaultForSecureChannels};
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    #[tokio::test]
    async fn test_encrypt_decrypt_normal_flow() {
        let (mut encryptor, mut decryptor) = create_encryptor_decryptor().await.unwrap();

        for n in 0..100 {
            let msg = vec![n];
            let mut ciphertext = Vec::new();
            encryptor.encrypt(&mut ciphertext, &msg).await.unwrap();
            assert_eq!(msg, decryptor.decrypt(&ciphertext).await.unwrap().0);
        }
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_with_message_lost() {
        let (mut encryptor, mut decryptor) = create_encryptor_decryptor().await.unwrap();

        for n in 0..100 {
            let msg = vec![n];
            let mut ciphertext = Vec::new();
            encryptor.encrypt(&mut ciphertext, &msg).await.unwrap();
            if n % 3 == 0 {
                // Two out of three packets are lost, but the ones that do reach the decryptor are
                // decrypted ok.
                assert_eq!(msg, decryptor.decrypt(&ciphertext).await.unwrap().0);
            }
        }
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_out_of_order() {
        let (mut encryptor, mut decryptor) = create_encryptor_decryptor().await.unwrap();

        // Vec<(plaintext, ciphertext)>
        let mut all_msgs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for n in 0..100 {
            let mut batch: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
            for m in 0..30 {
                let msg = vec![n, m];
                let mut ciphertext = Vec::new();
                encryptor.encrypt(&mut ciphertext, &msg).await.unwrap();
                batch.push((msg, ciphertext));
            }
            batch.shuffle(&mut thread_rng());
            all_msgs.append(&mut batch);
        }

        // Displaced up to 8 from the expected order, it is in the accepted window so all
        // must be decrypted ok.
        for (plaintext, ciphertext) in all_msgs.iter() {
            assert_eq!(plaintext, &decryptor.decrypt(ciphertext).await.unwrap().0);
        }
        // Repeated nonces are detected
        for (_plaintext, ciphertext) in all_msgs.iter() {
            assert!(decryptor.decrypt(ciphertext).await.is_err());
        }
        let msg = vec![1, 1];
        let mut ciphertext = Vec::new();
        encryptor.encrypt(&mut ciphertext, &msg).await.unwrap();
        // Good messages continue to be decrypted ok
        assert_eq!(msg, decryptor.decrypt(&ciphertext).await.unwrap().0);
    }

    #[tokio::test]
    async fn test_attack_nonce() {
        let (mut encryptor, mut decryptor) = create_encryptor_decryptor().await.unwrap();
        for n in 0..100 {
            let msg = vec![n];
            let mut ciphertext = Vec::new();
            encryptor.encrypt(&mut ciphertext, &msg).await.unwrap();
            let mut trash_packet = ciphertext.clone();
            // toggle a bit, to make the packet invalid.  The nonce is not affected
            // as it at the beginning of the packet
            trash_packet[ciphertext.len() - 1] ^= 0b1000_0000;

            // Generate a packet with some lookinly-valid content, but a nonce
            // far in the future that must be rejected.
            let mut bad_nonce_msg = Vec::new();
            let bad_nonce: u64 = 1000000;
            bad_nonce_msg.extend_from_slice(&bad_nonce.to_be_bytes());
            bad_nonce_msg.extend_from_slice(&ciphertext[8..]);

            assert!(decryptor.decrypt(&trash_packet).await.is_err());
            assert!(decryptor.decrypt(&bad_nonce_msg).await.is_err());
            // These invalid packets don't affect the decryptor state
            // FIXME: fix the implementation so this test pass.
            assert_eq!(msg, decryptor.decrypt(&ciphertext).await.unwrap().0);
        }
    }

    async fn create_encryptor_decryptor() -> Result<(Encryptor, Decryptor)> {
        let vault1 = SoftwareVaultForSecureChannels::create().await?;
        let vault2 = SoftwareVaultForSecureChannels::create().await?;

        let mut rng = thread_rng();
        let mut key = [0u8; 32];
        rng.fill_bytes(&mut key);

        let key_on_v1 = vault1.import_secret_buffer(key.to_vec()).await?;
        let key_on_v1 = vault1.convert_secret_buffer_to_aead_key(key_on_v1).await?;

        let key_on_v2 = vault2.import_secret_buffer(key.to_vec()).await?;
        let key_on_v2 = vault2.convert_secret_buffer_to_aead_key(key_on_v2).await?;

        Ok((
            Encryptor::new(key_on_v1, 0.into(), vault1, true),
            Decryptor::new(key_on_v2, vault2),
        ))
    }
}
