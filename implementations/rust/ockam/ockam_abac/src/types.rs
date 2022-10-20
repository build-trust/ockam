use either::Either;
use minicbor::decode::{self, Decoder};
use minicbor::encode::{self, Encoder, Write};
use minicbor::{Decode, Encode};
use ockam_core::compat::string::{String, ToString};
use str_buf::StrBuf;

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

        impl<'a, C> Decode<'a, C> for $t {
            fn decode(d: &mut Decoder<'a>, _: &mut C) -> Result<$t, decode::Error> {
                let s = d.str()?;
                Ok($t::new(s))
            }
        }
    };
}

define!(Subject);
define!(Resource);
define!(Action);
