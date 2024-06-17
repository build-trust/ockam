use either::Either;
use minicbor::decode::{self, Decoder};
use minicbor::encode::{self, Encoder, Write};
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::string::{String, ToString};
use serde::{Serialize, Serializer};
use str_buf::StrBuf;
use strum::{AsRefStr, Display, EnumIter, EnumString};

macro_rules! define {
    ($t:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $t(Either<String, StrBuf<{ Self::MAX_INLINE_SIZE }>>);

        impl $t {
            pub const MAX_INLINE_SIZE: usize = 24;

            pub const fn inline(s: &str) -> Option<Self> {
                if let Ok(s) = StrBuf::from_str_checked(s) {
                    Some(Self(Either::Right(s)))
                } else {
                    None
                }
            }

            pub const fn assert_inline(s: &str) -> Self {
                Self(Either::Right(StrBuf::from_str(s)))
            }

            pub fn new(s: &str) -> Self {
                if s.len() <= Self::MAX_INLINE_SIZE {
                    return Self::assert_inline(s);
                }
                Self(Either::Left(s.to_string()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $t {
            fn from(s: &str) -> Self {
                Self::new(s)
            }
        }

        impl From<String> for $t {
            fn from(s: String) -> Self {
                if s.len() <= Self::MAX_INLINE_SIZE {
                    return Self::assert_inline(&s);
                }
                Self(Either::Left(s))
            }
        }

        impl core::fmt::Display for $t {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl<C> Encode<C> for $t {
            fn encode<W>(
                &self,
                e: &mut Encoder<W>,
                _: &mut C,
            ) -> Result<(), encode::Error<W::Error>>
            where
                W: Write,
            {
                e.str(self.as_str())?.ok()
            }
        }

        impl<C> CborLen<C> for $t {
            fn cbor_len(&self, ctx: &mut C) -> usize {
                self.as_str().cbor_len(ctx)
            }
        }

        impl<'a, C> Decode<'a, C> for $t {
            fn decode(d: &mut Decoder<'a>, _: &mut C) -> Result<$t, decode::Error> {
                let s = d.str()?;
                Ok($t::new(s))
            }
        }

        impl Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }
    };
}

define!(Subject);
define!(ResourceName);

#[derive(
    Clone, Debug, Encode, Decode, CborLen, PartialEq, Eq, EnumString, Display, EnumIter, AsRefStr,
)]
#[cbor(index_only)]
pub enum Action {
    #[n(1)]
    #[strum(serialize = "handle_message")]
    HandleMessage,
}

impl Serialize for Action {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}
