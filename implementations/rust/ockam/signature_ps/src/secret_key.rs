use blake2::Blake2b;
use bls12_381_plus::Scalar;
use ff::Field;
use hkdf::HkdfExtract;
use rand_chacha::ChaChaRng;
use rand_core::{CryptoRng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use signature_core::lib::*;
use zeroize::Zeroize;

/// The secret key contains a field element for each
/// message that is signed and two extra.
/// See section 4.2 in
/// <https://eprint.iacr.org/2015/525.pdf> and
/// <https://eprint.iacr.org/2017/1197.pdf>
///
/// `w` corresponds to m' in the paper to achieve
/// EUF-CMA security level.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SecretKey {
    pub(crate) w: Scalar,
    pub(crate) x: Scalar,
    #[serde(with = "VecSerializer")]
    pub(crate) y: Vec<Scalar, 128>,
}

impl Zeroize for SecretKey {
    fn zeroize(&mut self) {
        self.w.zeroize();
        self.x.zeroize();
        for y in self.y.iter_mut() {
            y.zeroize();
        }
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Default for SecretKey {
    fn default() -> Self {
        Self {
            w: Scalar::zero(),
            x: Scalar::zero(),
            y: Vec::new(),
        }
    }
}

impl SecretKey {
    const SCALAR_SIZE: usize = 32;

    /// Compute a secret key from a hash
    pub fn hash<B: AsRef<[u8]>>(count: usize, data: B) -> Option<Self> {
        const SALT: &[u8] = b"PS-SIG-KEYGEN-SALT-";
        let info = (count as u32).to_be_bytes();
        let mut extractor = HkdfExtract::<Blake2b>::new(Some(SALT));
        extractor.input_ikm(data.as_ref());
        extractor.input_ikm(&[0u8]);
        let mut okm = [0u8; 32];
        let (_, h) = extractor.finalize();
        let _ = h.expand(&info[..], &mut okm);
        let rng = ChaChaRng::from_seed(okm);

        generate_secret_key(count, rng)
    }

    /// Compute a secret key from a CS-PRNG
    pub fn random(count: usize, rng: impl RngCore + CryptoRng) -> Option<Self> {
        generate_secret_key(count, rng)
    }

    /// Store the secret key as a sequence of bytes
    /// Each scalar is compressed to big-endian format
    /// Needs (N + 2) * 32 space otherwise it will panic
    /// where N is the number of messages that can be signed
    pub fn to_bytes(&self, buffer: &mut [u8]) {
        fn to_be_bytes(s: Scalar) -> [u8; 32] {
            let mut t = s.to_bytes();
            t.reverse();
            t
        }

        let mut offset = 0;
        let mut end = Self::SCALAR_SIZE;
        buffer[offset..end].copy_from_slice(&to_be_bytes(self.w)[..]);

        offset = end;
        end += Self::SCALAR_SIZE;

        buffer[offset..end].copy_from_slice(&to_be_bytes(self.x)[..]);

        offset = end;
        end += Self::SCALAR_SIZE;

        for y in &self.y {
            buffer[offset..end].copy_from_slice(&to_be_bytes(*y)[..]);
            offset = end;
            end += Self::SCALAR_SIZE;
        }
    }

    /// Convert a byte sequence into the secret key
    /// Expected size is (N + 2) * 32 bytes
    /// where N is the number of messages that can be signed
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Option<Self> {
        // Length for w, x, and 1 y
        const MIN_SIZE: usize = SecretKey::SCALAR_SIZE * 3;

        let buffer = bytes.as_ref();
        if buffer.len() % Self::SCALAR_SIZE != 0 {
            return None;
        }
        if buffer.len() < MIN_SIZE {
            return None;
        }

        fn from_be_bytes(d: &[u8]) -> Scalar {
            use core::convert::TryFrom;

            let mut t = <[u8; SecretKey::SCALAR_SIZE]>::try_from(d).expect("invalid length");
            t.reverse();
            Scalar::from_bytes(&t).unwrap()
        }

        let y_cnt = (buffer.len() / Self::SCALAR_SIZE) - 2;
        let mut offset = 0;
        let mut end = Self::SCALAR_SIZE;
        let w = from_be_bytes(&buffer[offset..end]);
        offset = end;
        end += Self::SCALAR_SIZE;

        let x = from_be_bytes(&buffer[offset..end]);
        offset = end;
        end += Self::SCALAR_SIZE;

        let mut y = Vec::new();

        for _ in 0..y_cnt {
            if y.push(from_be_bytes(&buffer[offset..end])).is_err() {
                return None;
            }
        }
        Some(Self { w, x, y })
    }

    /// Check if this secret key is valid
    pub fn is_valid(&self) -> bool {
        let mut res = !self.w.is_zero();
        res &= !self.x.is_zero();
        for y in &self.y {
            res &= !y.is_zero();
        }
        res
    }

    /// Check if this public key is invalid
    pub fn is_invalid(&self) -> bool {
        let mut res = self.w.is_zero();
        res |= self.x.is_zero();
        for y in &self.y {
            res |= y.is_zero();
        }
        res
    }
}

fn generate_secret_key(count: usize, mut rng: impl RngCore + CryptoRng) -> Option<SecretKey> {
    if count == 0 || count > 128 {
        return None;
    }
    let w = Scalar::random(&mut rng);
    let x = Scalar::random(&mut rng);
    let mut y = Vec::new();
    for _ in 0..count {
        if y.push(Scalar::random(&mut rng)).is_err() {
            return None;
        }
    }

    Some(SecretKey { w, x, y })
}
