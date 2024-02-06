use ockam_core::env::FromString;
use ockam_core::errcode::{Kind, Origin};
use std::str::FromStr;
use url::Url;

/// This struct can be used to parse environment variables representing URLs
pub struct UrlVar {
    pub(crate) url: Url,
}

impl UrlVar {
    pub fn new(url: Url) -> UrlVar {
        UrlVar { url }
    }
}

impl FromString for UrlVar {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        Ok(UrlVar {
            url: Url::from_str(s).map_err(|e| {
                ockam_core::Error::new(Origin::Api, Kind::Serialization, format!("{e:?}"))
            })?,
        })
    }
}
