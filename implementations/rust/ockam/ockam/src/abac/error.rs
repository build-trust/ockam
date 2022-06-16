//! Error types for the `abac` module.

use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Error types for attribute-based access control.
#[derive(Clone, Debug)]
pub enum AbacError {
    /// Invalid [`AbacLocalInfo`] type
    InvalidLocalInfoType = 1,
    /// Invalid [`AbacMetadata`] type
    InvalidMetadataType = 2,
    /// Abac trait storage read error,
    Read = 3,
    /// Abac trait storage write error,
    Write = 4,
}

impl From<AbacError> for Error {
    fn from(e: AbacError) -> Self {
        use AbacError::*;
        let kind = match e {
            InvalidLocalInfoType => Kind::Invalid,
            InvalidMetadataType => Kind::Invalid,
            Read => Kind::Io,
            Write => Kind::Io,
        };

        Self::new(Origin::Channel, kind, e)
    }
}

impl ockam_core::compat::error::Error for AbacError {}

impl core::fmt::Display for AbacError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLocalInfoType => "invalid AbacLocalInfo type".fmt(f),
            Self::InvalidMetadataType => "invalid AbacMetadata type".fmt(f),
            Self::Read => "storage read error".fmt(f),
            Self::Write => "storage write error".fmt(f),
        }
    }
}
