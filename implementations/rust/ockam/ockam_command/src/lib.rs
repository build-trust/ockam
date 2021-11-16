pub mod command;
pub mod config;
pub mod console;
pub mod spinner;

pub use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("unknown error")]
    Unknown,
    #[error("invalid command")]
    InvalidCommand,
}
