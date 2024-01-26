use miette::Diagnostic;
use serde::Serialize;
use thiserror::Error;

pub type Result<T> = miette::Result<T, Error>;

#[derive(Debug, Diagnostic, Error, Serialize)]
pub enum Error {
    #[error("creating enrollment ticket failed")]
    EnrollmentTicketFailed,
    #[error("decoding enrollment ticket failed")]
    EnrollmentTicketDecodeFailed,
    #[error("listing projects failed: {0:?}")]
    ListingFailed(String),
    #[error("application error: {0:?}")]
    InternalFailure(String),
    #[error("binary for ockam command is invalid: {0}")]
    OckamCommandInvalid(String),
    #[error("project {0} not found")]
    ProjectNotFound(String),
    #[error("failed to save local project state")]
    StateSaveFailed,
    #[error("{0}")]
    ProjectInvalidState(String),
}

impl From<Error> for String {
    #[track_caller]
    fn from(e: Error) -> Self {
        e.to_string()
    }
}

impl From<miette::Report> for Error {
    #[track_caller]
    fn from(e: miette::Report) -> Self {
        Error::InternalFailure(e.to_string())
    }
}
