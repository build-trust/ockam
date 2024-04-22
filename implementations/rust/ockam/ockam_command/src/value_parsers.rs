use crate::util::parsers::socket_addr_parser;
use miette::{miette, Context, IntoDiagnostic};
use ockam_api::cli_state::EnrollmentTicket;
use std::str::FromStr;
use url::Url;

/// Parse a single key-value pair
pub fn parse_key_val<T, U>(s: &str) -> miette::Result<(T, U)>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    U: FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| miette!("invalid key=value pair: no `=` found in `{s}`"))?;
    Ok((
        s[..pos].parse().into_diagnostic()?,
        s[pos + 1..].parse().into_diagnostic()?,
    ))
}

/// Parse an enrollment ticket given a path, a URL or hex-encoded string
pub fn parse_enrollment_ticket(value: &str) -> miette::Result<EnrollmentTicket> {
    let contents = parse_string_or_path_or_url(value)?;
    // Try to deserialize the contents as JSON
    if let Ok(enrollment_ticket) = serde_json::from_str(&contents) {
        Ok(enrollment_ticket)
    }
    // Try to decode the contents as hex
    else if let Ok(hex_decoded) = hex::decode(contents.trim()) {
        Ok(serde_json::from_slice(&hex_decoded)
            .into_diagnostic()
            .context("Failed to parse enrollment ticket from hex-encoded contents")?)
    } else {
        Err(miette!("Failed to parse enrollment ticket argument"))
    }
}

fn parse_string_or_path_or_url(value: &str) -> miette::Result<String> {
    parse_path_or_url(value).or_else(|_| Ok(value.to_string()))
}

pub fn parse_path_or_url(value: &str) -> miette::Result<String> {
    // If the URL is valid, download the contents
    if let Some(url) = is_url(value) {
        reqwest::blocking::get(url)
            .into_diagnostic()
            .context(format!("Failed to download file from {value}"))?
            .text()
            .into_diagnostic()
            .context("Failed to read contents from downloaded file")
    }
    // If not, try to read the contents from a file
    else if std::fs::metadata(value).is_ok() {
        std::fs::read_to_string(value)
            .into_diagnostic()
            .context("Failed to read contents from file")
    } else {
        Err(miette!("Failed to parse value {} as a path or URL", value))
    }
}

pub fn is_url(value: &str) -> Option<Url> {
    if let Ok(url) = Url::parse(value) {
        return Some(url);
    }
    // If the value is a socket address, try to parse it as a URL
    if let Some(socket_addr) = value.split('/').next() {
        if socket_addr.contains(':') && socket_addr_parser(socket_addr).is_ok() {
            let uri = format!("http://{value}");
            return Url::parse(&uri).ok();
        }
    }
    None
}

pub async fn async_parse_path_or_url(value: &str) -> miette::Result<String> {
    let value = value.to_string();
    tokio::task::spawn_blocking(move || parse_path_or_url(&value))
        .await
        .into_diagnostic()?
}
