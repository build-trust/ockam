use clap::error::{Error, ErrorKind};
use std::str::FromStr;
use std::time::Duration;

use miette::miette;

use ockam::identity::Identifier;
use ockam::transport::HostnamePort;
use ockam_api::config::lookup::InternetAddress;
use ockam_core::env::parse_duration;

use crate::util::validators::cloud_resource_name_validator;
use crate::Result;

/// Helper function for parsing a socket from user input
/// It is possible to just input a `port`. In that case the address will be assumed to be
/// 127.0.0.1:<port>
pub(crate) fn hostname_parser(input: &str) -> Result<HostnamePort> {
    Ok(HostnamePort::from_str(input)
        .map_err(|e| miette!("cannot parse the address {input} as a socket address: {e}"))?)
}

/// Helper fn for parsing an identifier from user input by using
/// [`ockam_identity::Identifier::from_str()`]
pub(crate) fn identity_identifier_parser(input: &str) -> Result<Identifier> {
    Ok(Identifier::from_str(input).map_err(|_| miette!("Invalid identity identifier: {input}"))?)
}

/// Helper fn for parsing an InternetAddress from user input by using
/// [`InternetAddress::new()`]
pub(crate) fn internet_address_parser(input: &str) -> Result<InternetAddress> {
    Ok(InternetAddress::new(input).ok_or_else(|| miette!("Invalid address: {input}"))?)
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
    parse_duration(arg).map_err(|_| Error::raw(ErrorKind::InvalidValue, "Invalid duration."))
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
