pub mod command;
pub mod config;
pub mod service;
pub mod spinner;

use std::io::Error;
pub use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("unknown error")]
    Unknown,
    #[error("invalid command")]
    InvalidCommand,
    #[error("invalid argument")]
    InvalidArgument,

    #[error("i/o error")]
    IoError(std::io::Error),

    #[error("ssh key error")]
    SshKey(ssh_key::Error),

    #[error("ockam error")]
    Ockam(ockam::Error),
}

impl From<ockam::Error> for AppError {
    fn from(ockam_error: ockam::Error) -> Self {
        AppError::Ockam(ockam_error)
    }
}

impl From<std::io::Error> for AppError {
    fn from(io_error: Error) -> Self {
        AppError::IoError(io_error)
    }
}

impl From<ssh_key::Error> for AppError {
    fn from(ssh_error: ssh_key::Error) -> Self {
        AppError::SshKey(ssh_error)
    }
}
