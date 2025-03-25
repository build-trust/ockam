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
use miette::Diagnostic;
pub use openapi::generate_schema;
use strum::Display;
use thiserror::Error;

#[derive(Debug, Display, Error, Diagnostic)]
pub enum ControlApiError {
    Response(ControlApiHttpResponse),
    Ockam(#[from] ockam_core::Error),
}

impl From<ControlApiHttpResponse> for ControlApiError {
    fn from(response: ControlApiHttpResponse) -> Self {
        Self::Response(response)
    }
}

macro_rules! impl_from_for_control_api_error {
    ($($err_type:ty),*) => {
        $(
            impl From<$err_type> for ControlApiError {
                fn from(error: $err_type) -> Self {
                    Self::Ockam(ockam_core::Error::from(error))
                }
            }
        )*
    };
}

impl_from_for_control_api_error!(
    CliStateError,
    ockam_multiaddr::Error,
    ockam_abac::ParseError,
    crate::error::ParseError
);
