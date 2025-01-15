use crate::CommandGlobalOpts;
use clap::crate_version;
use colorful::Colorful;
use miette::{miette, Error, IntoDiagnostic, Result, WrapErr};
use ockam_api::colors::{color_primary, color_uri};
use ockam_api::{fmt_log, fmt_warn};
use ockam_core::env::get_env_with_default;
use serde::Deserialize;
use std::env;
use std::fmt::Display;
use std::time::Duration;
use tracing::{debug, info, warn};
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

impl Display for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Release version: {}, download URL: {}",
            self.version, self.download_url
        )
    }
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

pub async fn check_if_an_upgrade_is_available(options: &CommandGlobalOpts) -> Result<()> {
    if upgrade_check_is_disabled() || options.global_args.test_argument_parser {
        debug!("Upgrade check is disabled");
        return Ok(());
    }

    let latest_release = get_release_data().await?;
    let current_version =
        semver::Version::parse(crate_version!()).map_err(|_| miette!("Invalid version"))?;
    let latest_version =
        semver::Version::parse(&latest_release.version).map_err(|_| miette!("Invalid version"))?;
    if current_version < latest_version {
        warn!(
            "A new version of the Ockam Command is now available: {}",
            latest_release.version
        );
        options.terminal.write_line(fmt_warn!(
            "A new version of the Ockam Command is now available: {}",
            color_primary(format!("v{}", latest_release.version))
        ))?;
        options.terminal.write_line(fmt_log!(
            "You can download it at: {}",
            color_uri(latest_release.download_url.as_ref())
        ))?;
        options.terminal.write_line(fmt_log!(
            "Or run the following command to upgrade it: {}\n",
            color_primary("brew install build-trust/ockam/ockam")
        ))?;
    } else {
        info!("The Ockam Command is up to date");
    }

    Ok(())
}

async fn get_release_data() -> Result<Release> {
    // All GitHub API requests must include a valid `User-Agent` header.
    // See https://docs.github.com/en/rest/using-the-rest-api/getting-started-with-the-rest-api?apiVersion=2022-11-28#user-agent
    let client = reqwest::Client::builder()
        .user_agent("ockam")
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            );
            headers
        })
        .timeout(Duration::from_secs(3))
        .build()
        .into_diagnostic()
        .wrap_err("Failed to create a HTTP client")?;
    let mut retries_left = 4;
    while retries_left > 0 {
        if let Ok(res) = client
            .get("https://github.com/build-trust/ockam/releases/latest")
            .send()
            .await
        {
            let json = res
                .json::<ReleaseJson>()
                .await
                .into_diagnostic()
                .wrap_err("Failed to parse JSON response")?;
            let parsed = Release::try_from(json)?;
            debug!(data=%parsed, "Got latest release data");
            return Ok(parsed);
        }
        warn!("Failed to retrieve the latest release data from GitHub, retrying...");
        retries_left -= 1;
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Err(miette!("Couldn't retrieve the release data from GitHub"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn parse_release_from_json_response() {
        let raw_response = r#"
            {"id":136375799,"tag_name":"ockam_v0.116.0",
            "update_url":"/build-trust/ockam/releases/tag/ockam_v0.116.0",
            "update_authenticity_token":"token",
            "delete_url":"/build-trust/ockam/releases/tag/ockam_v0.116.0",
            "delete_authenticity_token":"token",
            "edit_url":"/build-trust/ockam/releases/edit/ockam_v0.116.0"}
        "#;
        let json: ReleaseJson = serde_json::from_str(raw_response).unwrap();
        let release = Release::try_from(json).unwrap();
        assert_eq!(release.version, "0.116.0");
        assert_eq!(
            release.download_url,
            Url::parse("https://github.com/build-trust/ockam/releases/tag/ockam_v0.116.0").unwrap()
        );
    }

    #[test]
    fn parse_release_from_json_struct() {
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
        // Make sure that the data received from GitHub can be parsed correctly
        let mut is_ok = false;
        for _ in 0..5 {
            if get_release_data().await.is_ok() {
                is_ok = true;
                break;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        assert!(is_ok);
    }
}
