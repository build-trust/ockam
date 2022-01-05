pub mod command;
pub mod config;
pub mod spinner;

pub use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("unknown error")]
    Unknown,
    #[error("invalid command")]
    InvalidCommand,
    #[error("invalid argument")]
    InvalidArgument,

    #[error("ockam error")]
    Ockam(ockam::Error),
}

impl From<ockam::Error> for AppError {
    fn from(ockam_error: ockam::Error) -> Self {
        AppError::Ockam(ockam_error)
    }
}
