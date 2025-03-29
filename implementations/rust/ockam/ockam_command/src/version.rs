//! Helpers to display version information

use crate::branding::BrandingCompileEnvVars;
use clap::crate_version;
use ockam_api::colors::color_primary;
use ockam_api::output::Output;
use serde::Serialize;
use std::fmt::Display;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Version {
    #[serde(skip)]
    no_color: bool,
    #[serde(skip)]
    multiline: bool,
    version: String,
    hash: String,
}

impl Version {
    pub(crate) fn new() -> Self {
        Self {
            no_color: false,
            multiline: false,
            version: crate_version!().to_string(),
            hash: option_env!("GIT_HASH").unwrap_or("N/A").to_string(),
        }
    }

    pub(crate) fn no_color(mut self) -> Self {
        self.no_color = true;
        self
    }

    pub(crate) fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    pub(crate) fn clappy() -> &'static str {
        Box::leak(
            Self::new()
                .item()
                .expect("Failed to process version")
                .into_boxed_str(),
        )
    }

    fn prepare(&self) -> crate::Result<String> {
        let (version, hash) = if self.no_color {
            (self.version.clone(), self.hash.clone())
        } else {
            (
                color_primary(&self.version).to_string(),
                color_primary(&self.hash).to_string(),
            )
        };
        let bin_name = BrandingCompileEnvVars::bin_name();
        let msg = format!("{bin_name} {version}\ncompiled from {hash}");
        if self.multiline {
            Ok(msg)
        } else {
            Ok(msg.replace("\n", ", "))
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.prepare().expect("Failed to process version"))
    }
}

impl Output for Version {
    fn item(&self) -> ockam_api::Result<String> {
        Ok(self.padded_display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_mocked_version() -> Version {
        Version {
            no_color: false,
            multiline: false,
            version: "0.0.1".to_string(),
            hash: "e895fd75".to_string(),
        }
    }

    #[test]
    fn version_default() {
        let version = get_mocked_version().to_string();
        assert!(version.contains("0.0.1"));
        assert!(version.contains("e895fd75"));
        assert!(version.contains("\u{1b}[38;2;79;218;184m"));
    }

    #[test]
    fn version_no_color() {
        let version = get_mocked_version().no_color().to_string();
        assert!(version.contains("0.0.1"));
        assert!(version.contains("e895fd75"));
        assert!(!version.contains("\u{1b}[38;2;79;218;184m"));
    }

    #[test]
    fn version_multiline() {
        let version = get_mocked_version().multiline().to_string();
        assert!(version.contains("0.0.1"));
        assert!(version.contains("e895fd75"));
        assert_eq!(version.lines().count(), 2);
    }
}
