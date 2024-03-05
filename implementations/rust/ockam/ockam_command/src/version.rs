//! Helpers to display version information

use clap::crate_version;

pub(crate) struct Version;

impl Version {
    pub(crate) fn long() -> &'static str {
        Self::short()
    }

    pub(crate) fn short() -> &'static str {
        let crate_version = crate_version!();
        let git_hash = env!("GIT_HASH");
        let message = format!("{crate_version}\ncompiled from: {git_hash}");
        Box::leak(message.into_boxed_str())
    }
}
