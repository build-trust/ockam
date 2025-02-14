//! This module contains the implementation for the Control API and its protocol.
//!
//! The structures are completely independent, so changes outside this module cannot break
//! API compatibility.

pub mod backend;
pub mod frontend;
mod http;
mod openapi;
mod protocol;

use crate::cli_state::CliStateError;
use crate::control_api::http::ControlApiHttpResponse;
pub use openapi::generate_schema;

#[derive(Debug)]
pub enum ControlApiError {
    Response(ControlApiHttpResponse),
    OckamError(ockam_core::Error),
}
impl From<ControlApiHttpResponse> for ControlApiError {
    fn from(response: ControlApiHttpResponse) -> Self {
        Self::Response(response)
    }
}

impl From<ockam_core::Error> for ControlApiError {
    fn from(error: ockam_core::Error) -> Self {
        Self::OckamError(error)
    }
}

impl From<CliStateError> for ControlApiError {
    fn from(error: CliStateError) -> Self {
        Self::OckamError(ockam_core::Error::from(error))
    }
}

impl From<ockam_multiaddr::Error> for ControlApiError {
    fn from(error: ockam_multiaddr::Error) -> Self {
        Self::OckamError(ockam_core::Error::from(error))
    }
}

impl From<ockam_abac::ParseError> for ControlApiError {
    fn from(error: ockam_abac::ParseError) -> Self {
        Self::OckamError(ockam_core::Error::from(error))
    }
}
