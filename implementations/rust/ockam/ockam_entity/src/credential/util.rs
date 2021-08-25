use super::CredentialAttributeSchema;
use core::fmt;
use ockam_core::compat::{string::String, vec::Vec};

use serde::{
    de::{Error as DError, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserializer, Serializer,
};

#[cfg(test)]
pub struct MockRng(rand_xorshift::XorShiftRng);

#[cfg(test)]
impl rand::SeedableRng for MockRng {
    type Seed = [u8; 16];

    fn from_seed(seed: Self::Seed) -> Self {
        Self(rand_xorshift::XorShiftRng::from_seed(seed))
    }
}

#[cfg(test)]
impl rand::CryptoRng for MockRng {}

#[cfg(test)]
impl rand::RngCore for MockRng {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.0.try_fill_bytes(dest)
    }
}

#[allow(clippy::ptr_arg)]
pub fn write_attributes<S>(v: &Vec<CredentialAttributeSchema>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let l = if v.is_empty() { None } else { Some(v.len()) };

    let mut iter = s.serialize_seq(l)?;
    for i in v {
        iter.serialize_element(i)?;
    }
    iter.end()
}

pub fn read_attributes<'de, D>(deserializer: D) -> Result<Vec<CredentialAttributeSchema>, D::Error>
where
    D: Deserializer<'de>,
{
    struct BufferAttributeVisitor;

    impl<'de> Visitor<'de> for BufferAttributeVisitor {
        type Value = Vec<CredentialAttributeSchema>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("expected array of attributes")
        }

        fn visit_seq<A>(self, mut s: A) -> Result<Vec<CredentialAttributeSchema>, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let _l = if let Some(l) = s.size_hint() { l } else { 0 };
            let mut buf = Vec::new();
            while let Some(a) = s.next_element()? {
                let _result = buf.push(a);
                #[cfg(all(not(feature = "std"), not(feature = "alloc")))]
                {
                    _result.map_err(|_| DError::invalid_length(_l, &self))?;
                }
            }
            Ok(buf)
        }
    }

    deserializer.deserialize_seq(BufferAttributeVisitor)
}

#[allow(clippy::ptr_arg)]
pub fn write_byte_string<S>(v: &String, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(v.as_str())
}

pub fn read_byte_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct ByteStringVisitor;

    impl<'de> Visitor<'de> for ByteStringVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("expected array of attributes")
        }

        fn visit_str<E>(self, s: &str) -> Result<String, E>
        where
            E: DError,
        {
            Ok(String::from(s))
        }
    }

    deserializer.deserialize_str(ByteStringVisitor)
}
