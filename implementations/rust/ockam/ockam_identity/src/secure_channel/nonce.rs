use crate::IdentityError;
use core::fmt::{Display, Formatter};
use ockam_core::Result;

/// Nonce length for AES-GCM
pub const AES_GCM_NONCE_LEN: usize = 12;

/// Nonce length for Noise Protocol
pub const NOISE_NONCE_LEN: usize = 8;

/// Maximum possible Nonce value
pub const MAX_NONCE: Nonce = Nonce { value: u64::MAX };

/// Secure Channel Nonce
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Copy)]
pub struct Nonce {
    value: u64,
}

impl Display for Nonce {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<u64> for Nonce {
    fn from(value: u64) -> Self {
        Self { value }
    }
}

impl From<Nonce> for u64 {
    fn from(value: Nonce) -> Self {
        value.value
    }
}

impl Nonce {
    /// Constructor
    pub fn new(value: u64) -> Self {
        Self { value }
    }

    /// Nonce value
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Increment nonce value (overflow is not checked)
    pub fn increment(&mut self) -> Result<()> {
        if self == &MAX_NONCE {
            return Err(IdentityError::NonceOverflow)?;
        }

        self.value += 1;

        Ok(())
    }

    /// We use u64 nonce since it's convenient to work with it (e.g. increment)
    /// But we use 12-byte be format for encryption, since AES-GCM wants 12 bytes
    pub fn to_aes_gcm_nonce(&self) -> [u8; AES_GCM_NONCE_LEN] {
        let mut n: [u8; AES_GCM_NONCE_LEN] = [0; AES_GCM_NONCE_LEN];

        n[AES_GCM_NONCE_LEN - NOISE_NONCE_LEN..].copy_from_slice(&self.to_noise_nonce());

        n
    }

    /// We use u64 nonce since it's convenient to work with it (e.g. increment)
    /// But we use 8-byte be format to send it over to the other side (according to noise spec)
    pub fn to_noise_nonce(&self) -> [u8; NOISE_NONCE_LEN] {
        self.value.to_be_bytes()
    }
}

/// Restore 12-byte nonce needed for AES GCM from 8 byte that we use for noise
impl From<[u8; NOISE_NONCE_LEN]> for Nonce {
    fn from(value: [u8; NOISE_NONCE_LEN]) -> Self {
        let value = u64::from_be_bytes(value);

        Self { value }
    }
}

/// Restore 12-byte nonce needed for AES GCM from 8 byte that we use for noise
impl TryFrom<&[u8]> for Nonce {
    type Error = IdentityError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; NOISE_NONCE_LEN] =
            value.try_into().map_err(|_| IdentityError::InvalidNonce)?;

        Ok(bytes.into())
    }
}
