//! Helpers to display version information

use clap::crate_version;
use ockam_api::colors::color_primary;
use serde::ser::SerializeStruct;
use serde::Serialize;
use std::fmt::Display;

pub(crate) struct Version;

impl Version {
    pub(crate) fn long() -> &'static str {
        Self::short()
    }

    pub(crate) fn short() -> &'static str {
        let message = format!("{}\ncompiled from: {}", Self::version(), Self::hash());
        Box::leak(message.into_boxed_str())
    }

    fn version() -> &'static str {
        crate_version!()
    }

    fn hash() -> &'static str {
        option_env!("GIT_HASH").unwrap_or("N/A")
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, compiled from: {}",
            color_primary(Self::version()),
            color_primary(Self::hash())
        )
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Version", 2)?;
        state.serialize_field("version", Self::version())?;
        state.serialize_field("hash", &Self::hash())?;
        state.end()
    }
}
