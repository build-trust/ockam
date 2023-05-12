use crate::{KeyId, SecretKeyVec};
use core::fmt;
use minicbor::{Decode, Encode};
use ockam_core::hex_encoding;
use p256::elliptic_curve::subtle;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use zeroize::Zeroize;

/// Binary representation of a Secret.
#[derive(Serialize, Clone, Zeroize, Encode, Decode)]
#[zeroize(drop)]
#[cbor(transparent)]
pub struct Secret(
    #[serde(with = "hex_encoding")]
    #[n(0)]
    SecretKeyVec,
);

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        pub enum SecretV1 {
            Key(#[serde(with = "hex_encoding")] SecretKeyVec),
            Aws(KeyId),
        }
        #[derive(Deserialize)]
        struct SecretV2(#[serde(with = "hex_encoding")] SecretKeyVec);

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Secrets {
            V1(SecretV1),
            V2(SecretV2),
        }
        match Secrets::deserialize(deserializer) {
            Ok(Secrets::V1(SecretV1::Key(secret))) => Ok(Secret(secret)),
            Ok(Secrets::V1(SecretV1::Aws(_))) => {
                Err(D::Error::custom("AWS key ids are not supported anymore"))
            }
            Ok(Secrets::V2(SecretV2(secret))) => Ok(Secret(secret)),
            Err(e) => Err(e),
        }
    }
}

impl Secret {
    /// Create a new secret key.
    pub fn new(data: SecretKeyVec) -> Self {
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

    #[test]
    fn test_deserialize_legacy_secret_1() {
        let legacy = r#"{"Key":[1, 2, 3]}"#;
        let actual: Secret = de::from_str(legacy).unwrap();
        assert_eq!(actual, Secret(vec![1, 2, 3]));
    }

    #[test]
    fn test_deserialize_legacy_secret_2() {
        let legacy = r#"{"Key":"010203"}"#;
        let actual: Secret = de::from_str(legacy).unwrap();
        let expected = Secret(vec![1, 2, 3]);
        assert_eq!(actual, expected);
    }
}
