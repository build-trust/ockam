#![deny(missing_docs)]
#![allow(missing_docs)] // Contents are self describing for now.

use ockam_core::errcode::{Kind, Origin};
use std::{error::Error as StdError, fmt};

#[derive(Clone, Copy, Debug)]
pub enum SessionManagementError {
    MismatchedRequestType = 1,
    InvalidReceiverAddress,
    NoResponderRoute,
}

impl fmt::Display for SessionManagementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::MismatchedRequestType => "mismatched request type",
                Self::InvalidReceiverAddress => "invalid channel receiver address",
                Self::NoResponderRoute => "no response via the provided route",
            }
        )
    }
}

impl StdError for SessionManagementError {}

impl From<SessionManagementError> for ockam_core::Error {
    #[track_caller]
    fn from(e: SessionManagementError) -> ockam_core::Error {
        ockam_core::Error::new(Origin::Application, Kind::Misuse, e)
    }
}
