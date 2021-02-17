use ockam_core::lib::*;
use serde::{
    de::{SeqAccess, Visitor},
    ser::SerializeTuple,
    Deserialize, Deserializer, Serialize, Serializer,
};

/// The number of bytes in an HMAC SHA256 output
pub const HMAC_SHA256_SIZE: usize = 32;
/// The number of bytes in an ECDSA signature
pub const ECDSA_SIZE: usize = 64;
/// The number of bytes in a BBS+ signature
pub const BBS_PLUS_SIZE: usize = 112;

/// The signatures supported by a lease
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeaseSignature {
    /// Signature is an HMAC-SHA256
    HmacSha256([u8; HMAC_SHA256_SIZE]),
    /// Signature is ECDSA
    Ecdsa([u8; ECDSA_SIZE]),
    /// Signature is BBS+
    BbsPlus([u8; BBS_PLUS_SIZE]),
}

impl Serialize for LeaseSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        fn write_bytes<S>(v: u8, d: &[u8], s: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut t = s.serialize_tuple(d.len() + 1)?;
            t.serialize_element(&v)?;
            for b in d {
                t.serialize_element(b)?;
            }
            t.end()
        }

        match *self {
            LeaseSignature::HmacSha256(d) => write_bytes(1u8, &d[..], serializer),
            LeaseSignature::Ecdsa(d) => write_bytes(2u8, &d[..], serializer),
            LeaseSignature::BbsPlus(d) => write_bytes(3u8, &d[..], serializer),
        }
    }
}

impl<'de> Deserialize<'de> for LeaseSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TupleVisitor;

        fn read_from_seq<'de, A>(
            s: &dyn serde::de::Expected,
            out: &mut [u8],
            mut seq: A,
        ) -> Result<(), A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut count = 0;
            while let Some(b) = seq.next_element()? {
                out[count] = b;
                count += 1;
                if count == out.len() {
                    break;
                }
            }
            if count != out.len() {
                return Err(serde::de::Error::invalid_length(out.len(), s));
            }
            Ok(())
        }

        impl<'de> Visitor<'de> for TupleVisitor {
            type Value = LeaseSignature;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "expected array of bytes")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let variant = seq.next_element()?;
                match variant {
                    None => Err(serde::de::Error::invalid_length(1, &self)),
                    Some(variant) => match variant {
                        1u8 => {
                            let mut data = [0u8; HMAC_SHA256_SIZE];
                            read_from_seq(&self, &mut data, seq)?;
                            Ok(LeaseSignature::HmacSha256(data))
                        }
                        2u8 => {
                            let mut data = [0u8; ECDSA_SIZE];
                            read_from_seq(&self, &mut data, seq)?;
                            Ok(LeaseSignature::Ecdsa(data))
                        }
                        3u8 => {
                            let mut data = [0u8; BBS_PLUS_SIZE];
                            read_from_seq(&self, &mut data, seq)?;
                            Ok(LeaseSignature::BbsPlus(data))
                        }
                        _ => Err(serde::de::Error::invalid_length(1, &self)),
                    },
                }
            }
        }

        deserializer.deserialize_tuple(BBS_PLUS_SIZE + 1, TupleVisitor)
    }
}

#[test]
fn test_serialization() {
    let signatures = [
        LeaseSignature::HmacSha256([1u8; 32]),
        LeaseSignature::Ecdsa([2u8; 64]),
        LeaseSignature::BbsPlus([3u8; 112]),
    ];
    for s in &signatures {
        // test json
        let res = serde_json::to_string(s);
        assert!(res.is_ok());
        let json = res.unwrap();
        let res = serde_json::from_str::<LeaseSignature>(&json);
        assert!(res.is_ok());
        let sig = res.unwrap();
        assert_eq!(*s, sig);

        let res = serde_bare::to_vec(s);
        assert!(res.is_ok());
        let bare = res.unwrap();
        let res = serde_bare::from_slice::<LeaseSignature>(&bare);
        assert!(res.is_ok());
        let sig = res.unwrap();
        assert_eq!(*s, sig);
    }
}
