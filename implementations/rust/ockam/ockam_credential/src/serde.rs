use super::{structs::*, Attribute};
use serde::{
    de::{Error as DError, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserializer, Serializer,
};

#[allow(clippy::ptr_arg)]
pub fn write_attributes<S>(v: &Buffer<Attribute>, s: S) -> Result<S::Ok, S::Error>
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

pub fn read_attributes<'de, D>(deserializer: D) -> Result<Buffer<Attribute>, D::Error>
where
    D: Deserializer<'de>,
{
    struct BufferAttributeVisitor;

    impl<'de> Visitor<'de> for BufferAttributeVisitor {
        type Value = Buffer<Attribute>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("expected array of attributes")
        }

        fn visit_seq<A>(self, mut s: A) -> Result<Buffer<Attribute>, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let _l = if let Some(l) = s.size_hint() { l } else { 0 };
            let mut buf = Buffer::new();
            while let Some(a) = s.next_element()? {
                #[cfg(all(feature = "no_std", not(feature = "alloc")))]
                {
                    buf.push(a).map_err(|_| DError::invalid_length(_l, &self))?;
                }
                #[cfg(feature = "alloc")]
                {
                    buf.push(a);
                }
            }
            Ok(buf)
        }
    }

    deserializer.deserialize_seq(BufferAttributeVisitor)
}

#[allow(clippy::ptr_arg)]
pub fn write_byte_string<S>(v: &ByteString, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(v.as_str())
}

pub fn read_byte_string<'de, D>(deserializer: D) -> Result<ByteString, D::Error>
where
    D: Deserializer<'de>,
{
    struct ByteStringVisitor;

    impl<'de> Visitor<'de> for ByteStringVisitor {
        type Value = ByteString;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("expected array of attributes")
        }

        fn visit_str<E>(self, s: &str) -> Result<ByteString, E>
        where
            E: DError,
        {
            Ok(ByteString::from(s))
        }
    }

    deserializer.deserialize_str(ByteStringVisitor)
}
