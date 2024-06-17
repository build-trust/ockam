use minicbor::{decode, encode, CborLen, Decode, Decoder, Encode, Encoder};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;
use url::{ParseError, Url as RegularUrl};

/// Adding newtype for url::Url, since we can't use derive macro for external types
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Url(RegularUrl);

impl Url {
    pub fn new(url: RegularUrl) -> Self {
        Self(url)
    }

    pub fn parse(input: &str) -> Result<Url, ParseError> {
        let url = RegularUrl::parse(input)?;
        Ok(Url(url))
    }
}

impl Deref for Url {
    type Target = RegularUrl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl<C> Encode<C> for Url {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        self.0.as_str().encode(e, ctx)
    }
}

impl<C> CborLen<C> for Url {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        self.0.as_str().cbor_len(ctx)
    }
}

impl<'b, C> Decode<'b, C> for Url {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, decode::Error> {
        let url = d.decode_with(ctx)?;
        let url = RegularUrl::from_str(url).map_err(decode::Error::message)?;
        Ok(Url(url))
    }
}

impl Serialize for Url {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.as_str())
    }
}

impl<'d> Deserialize<'d> for Url {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        let url: &str = Deserialize::deserialize(deserializer)?;
        let url = RegularUrl::from_str(url).map_err(de::Error::custom)?;
        Ok(Url(url))
    }
}

impl From<Url> for RegularUrl {
    fn from(url: Url) -> Self {
        url.0
    }
}
