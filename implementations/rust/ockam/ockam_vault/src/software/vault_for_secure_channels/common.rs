use crate::{AeadSecretKeyHandle, HandleToSecret, SecretBufferHandle};
use alloc::vec;
use ockam_core::compat::rand::{thread_rng, RngCore};

pub(super) fn generate_random_handle() -> HandleToSecret {
    // NOTE: Buffer and Aes secrets in the system are ephemeral and it should be fine,
    // that every time we import the same secret - it gets different Handle value.
    // However, if we decide to have persistent Buffer or Aes secrets, that should be
    // changed (probably to hash value of the secret)
    let mut rng = thread_rng();
    let mut rand = vec![0u8; 8];
    rng.fill_bytes(&mut rand);
    HandleToSecret::new(rand)
}

pub(super) fn generate_buffer_handle() -> SecretBufferHandle {
    SecretBufferHandle(generate_random_handle())
}

pub(super) fn generate_aead_handle() -> AeadSecretKeyHandle {
    use crate::Aes256GcmSecretKeyHandle;
    let handle = generate_random_handle();
    AeadSecretKeyHandle(Aes256GcmSecretKeyHandle(handle))
}
