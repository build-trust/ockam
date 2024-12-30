use clap::error::{Error, ErrorKind};
use std::str::FromStr;
use std::time::Duration;

use miette::{miette, WrapErr};

use ockam::identity::Identifier;
use ockam::transport::SchemeHostnamePort;
use ockam_api::config::lookup::InternetAddress;
use ockam_core::env::parse_duration;

//
use crate::util::validators::cloud_resource_name_validator;
use crate::Result;

/// Helper function for parsing a socket from user input by using
/// [`SchemeHostnamePort::from_str()`]
pub(crate) fn hostname_parser(input: &str) -> Result<SchemeHostnamePort> {
    SchemeHostnamePort::from_str(input).wrap_err(format!(
        "cannot parse the address {input} as a socket address"
    ))
}

/// Helper function for parsing a key-value pair from user input
/// The input is expected to be in the format `key:value`
pub(crate) fn http_header_parser(input: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 2 {
        return Err(miette!("Invalid header format. Expected 'key:value'"));
    }

    Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
}

/// Helper fn for parsing an identifier from user input by using
/// [`ockam_identity::Identifier::from_str()`]
pub(crate) fn identity_identifier_parser(input: &str) -> Result<Identifier> {
    Identifier::from_str(input).wrap_err(format!("Invalid identity identifier: {input}"))
}

/// Helper fn for parsing an InternetAddress from user input by using
/// [`InternetAddress::new()`]
pub(crate) fn internet_address_parser(input: &str) -> Result<InternetAddress> {
    InternetAddress::new(input).ok_or_else(|| miette!("Invalid address: {input}"))
}

pub(crate) fn project_name_parser(s: &str) -> Result<String> {
    match cloud_resource_name_validator(s) {
        Ok(_) => Ok(s.to_string()),
        Err(_e)=> Err(miette!(
            "project name can contain only alphanumeric characters and the '-', '_' and '.' separators. \
            Separators must occur between alphanumeric characters. This implies that separators can't \
            occur at the start or end of the name, nor they can occur in sequence.",
        ))?,
    }
}

pub(crate) fn duration_parser(arg: &str) -> std::result::Result<Duration, clap::Error> {
    parse_duration(arg).map_err(|_| Error::raw(ErrorKind::InvalidValue, "Invalid duration"))
}

pub(crate) fn duration_to_human_format(duration: &Duration) -> String {
    let mut parts = vec![];
    let secs = duration.as_secs();
    let days = secs / 86400;
    if days > 0 {
        parts.push(format!("{}d", days));
    }
    let hours = (secs % 86400) / 3600;
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    let minutes = (secs % 3600) / 60;
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    let seconds = secs % 60;
    if seconds > 0 {
        parts.push(format!("{}s", seconds));
    }
    parts.join(" ")
}

#[cfg(test)]
mod test {
    use crate::util::parsers::http_header_parser;

    #[test]
    pub fn test_http_header_parser() {
        assert_eq!(
            http_header_parser("key:value").unwrap(),
            ("key".to_string(), "value".to_string())
        );
        assert_eq!(
            http_header_parser("key: value").unwrap(),
            ("key".to_string(), "value".to_string())
        );
        assert!(http_header_parser("key:value:extra").is_err());
        assert!(http_header_parser("key").is_err());
    }
}
