use crate::colors::OckamColor;
use crate::error::ParseError;
use crate::output::Output;
use crate::Result;
use colorful::Colorful;
use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use time::format_description::well_known::Iso8601;
use time::PrimitiveDateTime;

#[derive(Encode, Decode, CborLen, Serialize, Deserialize, Debug)]
#[cbor(map)]
pub struct Token {
    #[cbor(n(1))]
    pub id: String,

    #[cbor(n(2))]
    pub issued_for: String,

    #[cbor(n(3))]
    pub created_at: String,

    #[cbor(n(4))]
    pub expires: String,

    #[cbor(n(5))]
    pub token: String,

    #[cbor(n(6))]
    pub status: String,
}

impl Output for Token {
    fn item(&self) -> Result<String> {
        let mut output = String::new();
        let status = match self.status.as_str() {
            "active" => self
                .status
                .to_uppercase()
                .color(OckamColor::Success.color()),
            _ => self
                .status
                .to_uppercase()
                .color(OckamColor::Failure.color()),
        };
        let expires_at = {
            PrimitiveDateTime::parse(&self.expires, &Iso8601::DEFAULT)
                .map_err(ParseError::Time)?
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        };
        let id = self
            .id
            .to_string()
            .color(OckamColor::PrimaryResource.color());

        writeln!(output, "Token {id}")?;
        writeln!(output, "Expires {expires_at} {status}")?;
        write!(output, "{}", self.token)?;

        Ok(output)
    }
}
