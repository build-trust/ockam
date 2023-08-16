use serde::Serialize;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, Serialize)]
pub enum Error {
    #[error("listing projects failed: {0:?}")]
    ListingFailed(ockam::Error),
    #[error("failed to save local project state")]
    StateSaveFailed,
}

impl From<Error> for String {
    fn from(e: Error) -> Self {
        e.to_string()
    }
}
