use bls12_381_plus::Scalar;
use hkdf::HkdfExtract;
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use subtle::CtOption;
use zeroize::Zeroize;

/// The secret key is field element 0 < `x` < `r`
/// where `r` is the curve order. See Section 4.3 in
/// <https://eprint.iacr.org/2016/663.pdf>
#[derive(Clone, Debug, Eq, PartialEq, Zeroize)]
#[zeroize(drop)]
pub struct SecretKey(pub Scalar);

impl Default for SecretKey {
    fn default() -> Self {
        Self(Scalar::zero())
    }
}

impl From<SecretKey> for [u8; SecretKey::BYTES] {
    fn from(sk: SecretKey) -> [u8; SecretKey::BYTES] {
        sk.to_bytes()
    }
}

impl<'a> From<&'a SecretKey> for [u8; SecretKey::BYTES] {
    fn from(sk: &'a SecretKey) -> [u8; SecretKey::BYTES] {
        sk.to_bytes()
    }
}

impl Serialize for SecretKey {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for SecretKey {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let scalar = Scalar::deserialize(d)?;
        Ok(Self(scalar))
    }
}

impl SecretKey {
    /// Number of bytes needed to represent the secret key
    pub const BYTES: usize = 32;

    /// Compute a secret key from a hash
    pub fn hash<B: AsRef<[u8]>>(data: B) -> Option<Self> {
        generate_secret_key(data.as_ref())
    }

    /// Compute a secret key from a CS-PRNG
    pub fn random(mut rng: impl RngCore + CryptoRng) -> Option<Self> {
        let mut data = [0u8; Self::BYTES];
        rng.fill_bytes(&mut data);
        generate_secret_key(&data)
    }

    /// Get the byte representation of this key
    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        let mut bytes = self.0.to_bytes();
        // Make big endian
        bytes.reverse();
        bytes
    }

    /// Convert a big-endian representation of the secret key.
    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> CtOption<Self> {
        let mut t = [0u8; Self::BYTES];
        t.copy_from_slice(bytes);
        t.reverse();
        Scalar::from_bytes(&t).map(SecretKey)
    }
}

fn generate_secret_key(ikm: &[u8]) -> Option<SecretKey> {
    const SALT: &[u8] = b"BLS-SIG-KEYGEN-SALT-";
    const INFO: [u8; 2] = [0u8, 48u8];

    let mut extracter = HkdfExtract::<sha2::Sha256>::new(Some(SALT));
    extracter.input_ikm(ikm);
    extracter.input_ikm(&[0u8]);
    let (_, h) = extracter.finalize();

    let mut output = [0u8; 48];
    if h.expand(&INFO, &mut output).is_err() {
        None
    } else {
        Some(SecretKey(Scalar::from_okm(&output)))
    }
}
