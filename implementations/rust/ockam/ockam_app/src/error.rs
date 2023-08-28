use miette::Diagnostic;
use thiserror::Error;

pub type Result<T> = miette::Result<T, Error>;

#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error("{0}")]
    Generic(String),

    #[error(transparent)]
    Ockam(#[from] ockam_core::Error),

    #[error(transparent)]
    Api(#[from] ockam_api::error::ApiError),

    #[error(transparent)]
    CliState(#[from] ockam_api::cli_state::CliStateError),

    #[error(transparent)]
    Tauri(#[from] tauri::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    JsonSerde(#[from] serde_json::Error),
}

impl From<miette::Report> for Error {
    fn from(e: miette::Report) -> Self {
        Error::Generic(e.to_string())
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Error::Generic(e.to_string())
    }
}
