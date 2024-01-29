use crate::terminal::{color_primary, color_uri};
use crate::{fmt_log, fmt_warn, CommandGlobalOpts};
use clap::crate_version;
use colorful::Colorful;
use miette::{miette, Error, IntoDiagnostic, Result, WrapErr};
use ockam_core::env::get_env_with_default;
use serde::Deserialize;
use std::env;
use tracing::{debug, warn};
use url::Url;

const RELEASE_TAG_NAME_PREFIX: &str = "ockam_v";

fn upgrade_check_is_disabled() -> bool {
    get_env_with_default("OCKAM_DISABLE_UPGRADE_CHECK", false).unwrap_or(false)
}

#[derive(Deserialize, Debug)]
struct ReleaseJson {
    tag_name: String,
    update_url: String,
}

impl ReleaseJson {
    fn version(&self) -> Result<String> {
        self.tag_name
            .split_once(RELEASE_TAG_NAME_PREFIX)
            .ok_or(miette!("Unknown release version: {}", self.tag_name))
            .map(|(_, version)| version.to_string())
    }

    fn update_url(&self) -> Result<Url> {
        Url::options()
            .base_url(Some(&Url::parse("https://github.com").into_diagnostic()?))
            .parse(&self.update_url)
            .into_diagnostic()
            .wrap_err(format!("Invalid download URL: {}", self.update_url))
    }
}

struct Release {
    version: String,
    download_url: Url,
}

impl TryFrom<ReleaseJson> for Release {
    type Error = Error;

    fn try_from(json: ReleaseJson) -> Result<Self> {
        Ok(Self {
            version: json.version()?,
            download_url: json.update_url()?,
        })
    }
}

pub fn check_if_an_upgrade_is_available(options: &CommandGlobalOpts) -> Result<()> {
    if upgrade_check_is_disabled() || options.global_args.test_argument_parser {
        debug!("Upgrade check is disabled");
        return Ok(());
    }

    let latest_release = get_release_data()?;
    if crate_version!() != latest_release.version {
        warn!(
            "A new version of the Ockam Command is available: {}",
            latest_release.version
        );
        options.terminal.write_line(fmt_warn!(
            "A new version is now available: {}",
            color_primary(format!("v{}", crate_version!()))
        ))?;
        options.terminal.write_line(fmt_log!(
            "You can download it at: {}",
            color_uri(latest_release.download_url.as_ref())
        ))?;
        options.terminal.write_line(fmt_log!(
            "Or run the following command to upgrade it: {}\n",
            color_primary("brew install build-trust/ockam/ockam")
        ))?;
    }

    Ok(())
}

fn get_release_data() -> Result<Release> {
    // All GitHub API requests must include a valid `User-Agent` header.
    // See https://docs.github.com/en/rest/using-the-rest-api/getting-started-with-the-rest-api?apiVersion=2022-11-28#user-agent
    let client = reqwest::blocking::Client::builder()
        .user_agent("ockam")
        .build()
        .into_diagnostic()?;
    let parsed = client
        .get("https://github.com/build-trust/ockam/releases/latest")
        .send()
        .and_then(|resp| resp.json::<ReleaseJson>())
        .into_diagnostic()?;
    Release::try_from(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_retry::strategy::{jitter, FixedInterval};
    use tokio_retry::Retry;

    #[test]
    fn parse_release_from_json_data() {
        let crate_version = crate_version!().to_string();

        // Expected version format
        let json = ReleaseJson {
            tag_name: "ockam_v0.116.0".to_string(),
            update_url: "/build-trust/ockam/releases/tag/ockam_v0.116.0".to_string(),
        };
        let release = Release::try_from(json).unwrap();
        assert_eq!(release.version, "0.116.0");
        assert_eq!(
            release.download_url,
            Url::parse("https://github.com/build-trust/ockam/releases/tag/ockam_v0.116.0").unwrap()
        );

        let json = ReleaseJson {
            tag_name: format!("{RELEASE_TAG_NAME_PREFIX}{crate_version}"),
            update_url: "/build-trust/ockam/releases/tag/ockam_v0.116.0".to_string(),
        };
        let release = Release::try_from(json).unwrap();
        assert_eq!(&release.version, &crate_version);
        assert_eq!(
            release.download_url,
            Url::parse("https://github.com/build-trust/ockam/releases/tag/ockam_v0.116.0").unwrap()
        );

        // Unexpected version format will fail
        let json = ReleaseJson {
            tag_name: "unknown_v0.0.1".to_string(),
            update_url: "/build-trust/ockam/releases/tag/ockam_v0.116.0".to_string(),
        };
        assert!(Release::try_from(json).is_err());
    }

    #[tokio::test]
    async fn get_and_parse_release_data_from_github() {
        // Make sure that the data received from GitHub is can be parsed correctly
        let retry_strategy = FixedInterval::from_millis(5000).map(jitter).take(5);
        Retry::spawn(retry_strategy, || async {
            get_release_data().map_err(|e| {
                eprintln!("Failed to get release data: {e:?}");
                e
            })
        });
    }
}
