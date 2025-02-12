use crate::node::config::NodeConfig;
use crate::util::parsers::hostname_parser;
use crate::CommandGlobalOpts;
use miette::{miette, Context, IntoDiagnostic};
use ockam_api::cli_state::{EnrollmentTicket, ExportedEnrollmentTicket, LegacyEnrollmentTicket};
use ockam_api::colors::color_primary;
use std::str::FromStr;
use tracing::{trace, warn};
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

/// Parse an enrollment ticket given a path, a URL or encoded string
pub async fn parse_enrollment_ticket(
    _opts: &CommandGlobalOpts,
    value: &str,
) -> miette::Result<EnrollmentTicket> {
    trace!(%value, "parsing enrollment ticket");
    let contents = read_contents_from_string_or_path_or_url(value).await?;

    // Try to parse it using the old format
    if let Ok(ticket) = LegacyEnrollmentTicket::from_str(&contents) {
        return Ok(EnrollmentTicket::new_from_legacy(ticket).await?);
    }

    Ok(ExportedEnrollmentTicket::from_str(&contents)?
        .import()
        .await?)
}

pub(crate) async fn read_config_contents_from_path_or_url_or_inline(
    value: &str,
) -> miette::Result<String> {
    let contents = match read_contents_from_path_or_url(value).await {
        Ok(contents) => contents,
        Err(_) => value.to_string(),
    };
    let parsed = serde_yaml::from_str::<NodeConfig>(&contents);
    if !contents.is_empty() && parsed.is_ok() {
        Ok(contents)
    } else {
        Err(miette!(
            "Failed to parse configuration from value {} as a path, URL or inline contents",
            color_primary(value)
        ))
    }
}

pub(crate) async fn read_contents_from_string_or_path_or_url(
    value: &str,
) -> miette::Result<String> {
    read_contents_from_path_or_url(value).await.or_else(|err| {
        warn!(%value, %err, "Couldn't parse value as a path or URL. Returning plain value to be processed as inline contents");
        Ok(value.to_string())
    })
}

pub(crate) async fn read_contents_from_path_or_url(value: &str) -> miette::Result<String> {
    // If the URL is valid, download the contents
    if let Some(url) = is_url(value) {
        reqwest::get(url)
            .await
            .into_diagnostic()
            .context(format!("Failed to download file from {value}"))?
            .text()
            .await
            .into_diagnostic()
            .context("Failed to read contents from downloaded file")
    }
    // Try to read the contents from a file
    else if tokio::fs::metadata(value).await.is_ok() {
        tokio::fs::read_to_string(value)
            .await
            .into_diagnostic()
            .context("Failed to read contents from file")
    } else {
        warn!(%value, "Couldn't parse value as a path or URL");
        Err(miette!(
            "Couldn't parse value {} as a path or URL",
            color_primary(value)
        ))
    }
}

pub fn is_url(value: &str) -> Option<Url> {
    if let Ok(url) = Url::parse(value) {
        return Some(url);
    }
    // If the value is a socket address, try to parse it as a URL
    if let Some(socket_addr) = value.split('/').next() {
        if socket_addr.contains(':') && hostname_parser(socket_addr).is_ok() {
            let uri = format!("http://{value}");
            return Url::parse(&uri).ok();
        }
    }
    None
}
