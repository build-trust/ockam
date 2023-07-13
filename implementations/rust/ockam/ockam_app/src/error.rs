use miette::Diagnostic;
use thiserror::Error;

pub type TauriCommandResult<T> = std::result::Result<T, String>;

pub type Result<T> = miette::Result<T, Error>;

#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error("{0}")]
    Generic(String),

    #[error(transparent)]
    Command(#[from] ockam_command::error::Error),

    #[error(transparent)]
    CommandState(#[from] ockam_api::cli_state::CliStateError),

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

impl From<std::fmt::Error> for Error {
    fn from(e: std::fmt::Error) -> Self {
        Error::Generic(e.to_string())
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Error::Generic(e)
    }
}
