use core::fmt;
use minicbor::{Decode, Encode};
use ockam_core::compat::string::String;
use ockam_core::compat::vec::Vec;
use ockam_core::hex_encoding;
use p256::elliptic_curve::subtle;
use serde::{Deserialize, Deserializer, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// KeyId.
pub type KeyId = String;

/// Binary representation of a Secret.
#[derive(Serialize, Clone, Zeroize, ZeroizeOnDrop, Encode, Decode)]
#[cbor(transparent)]
pub struct Secret(
    #[serde(with = "hex_encoding")]
    #[n(0)]
    Vec<u8>,
);

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SecretV2(#[serde(with = "hex_encoding")] Vec<u8>);

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Secrets {
            V2(SecretV2),
        }
        match Secrets::deserialize(deserializer) {
            Ok(Secrets::V2(SecretV2(secret))) => Ok(Secret(secret)),
            Err(e) => Err(e),
        }
    }
}

impl Secret {
    /// Create a new secret key.
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    /// Return the secret length
    pub fn length(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("<secret key omitted>")
    }
}

impl Eq for Secret {}

impl PartialEq for Secret {
    fn eq(&self, o: &Self) -> bool {
        subtle::ConstantTimeEq::ct_eq(&self.0[..], &o.0[..]).into()
    }
}

impl AsRef<[u8]> for Secret {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::de;

    #[test]
    fn test_serialize_secret() {
        let secret = Secret(vec![1, 2, 3]);
        let actual: String = serde_json::to_string(&secret).unwrap();
        assert_eq!(actual, "\"010203\"".to_string());
    }

    #[test]
    fn test_deserialize_secret() {
        let actual: Secret = de::from_str("\"010203\"").unwrap();
        assert_eq!(actual, Secret(vec![1, 2, 3]));
    }
}
