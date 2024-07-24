use core::fmt;
use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

use crate::colors::{color_error, color_ok};
use crate::error::ApiError;

use ockam_core::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, CborLen, Serialize, Deserialize)]
#[rustfmt::skip]
pub enum ConnectionStatus {
    #[n(0)] Down,
    #[n(1)] Up,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Down
    }
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Down => write!(f, "{}", color_error("DOWN")),
            ConnectionStatus::Up => write!(f, "{}", color_ok("UP")),
        }
    }
}

impl TryFrom<String> for ConnectionStatus {
    type Error = ApiError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "down" => Ok(ConnectionStatus::Down),
            "up" => Ok(ConnectionStatus::Up),
            _ => Err(ApiError::message(format!(
                "Invalid connection status: {value}"
            ))),
        }
    }
}
