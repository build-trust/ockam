//! Helpers to display version information

use clap::crate_version;
use ockam_core::env::get_env_with_default;

pub(crate) struct Version;

impl Version {
    pub(crate) fn long() -> &'static str {
        Self::short()
    }

    pub(crate) fn short() -> &'static str {
        let crate_version = crate_version!();
        let na_hash = "N/A".to_string();
        let git_hash = get_env_with_default("GIT_HASH", na_hash.clone()).unwrap_or(na_hash);
        let message = format!("{crate_version}\ncompiled from: {git_hash}");
        Box::leak(message.into_boxed_str())
    }
}
