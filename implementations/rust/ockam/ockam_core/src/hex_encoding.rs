use alloc::vec::Vec;
use core::fmt;
use serde::de::{SeqAccess, Unexpected};
use serde::{Deserializer, Serializer};

/// By default, serde serializes using a sequence of integers.
/// We rather serialize a using hex string.
pub fn serialize<S: Serializer>(value: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&hex::encode(value.as_slice()))
}

/// To keep back-compatibility we parse both sequence of integer or a hex string
pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("secret key")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match hex::decode(value.as_bytes()) {
                Ok(decoded) => Ok(decoded),
                Err(_) => {
                    return Err(serde::de::Error::invalid_value(
                        Unexpected::Other("invalid hex"),
                        &self,
                    ))
                }
            }
        }

        // legacy format was a sequence of integers
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut buffer = Vec::with_capacity(32);
            while let Some(value) = seq.next_element()? {
                buffer.push(value);
            }
            Ok(buffer)
        }
    }
    deserializer.deserialize_any(Visitor {})
}
