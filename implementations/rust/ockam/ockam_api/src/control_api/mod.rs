//! This module contains the implementation for the Control API and its protocol.
//!
//! The structures are completely independent, so changes outside this module cannot break
//! API compatibility.

use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use serde::{Deserialize, Serialize};

pub mod backend;
pub mod frontend;

mod protocol;

#[derive(Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub(crate) struct ControlApiHttpRequest {
    #[n(0)] pub(super) method: String,
    #[n(1)] pub(super) uri: String,
    #[n(2)] pub(super) body: Option<Vec<u8>>,
}

#[derive(Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub(crate) struct ControlApiHttpResponse {
    #[n(0)] pub(super) status: u16,
    #[n(1)] pub(super) body: Vec<u8>,
}

impl ControlApiHttpResponse {
    fn with_body<T: Serialize>(
        status_code: StatusCode,
        body: T,
    ) -> ockam_core::Result<ControlApiHttpResponse> {
        Ok(Self {
            status: status_code.as_u16(),
            body: serde_json::to_vec(&body).map_err(|_| {
                Error::new(
                    Origin::Api,
                    Kind::Internal,
                    "Failed to encode response body",
                )
            })?,
        })
    }

    fn without_body(status_code: StatusCode) -> ockam_core::Result<ControlApiHttpResponse> {
        Ok(Self {
            status: status_code.as_u16(),
            body: Vec::new(),
        })
    }

    fn invalid_body() -> ockam_core::Result<ControlApiHttpResponse> {
        Self::with_body(
            StatusCode::BAD_REQUEST,
            ErrorResponse {
                message: "Invalid request body".to_string(),
            },
        )
    }

    fn missing_body() -> ockam_core::Result<ControlApiHttpResponse> {
        Self::with_body(
            StatusCode::BAD_REQUEST,
            ErrorResponse {
                message: "Missing request body".to_string(),
            },
        )
    }

    fn internal_error() -> ockam_core::Result<ControlApiHttpResponse> {
        Self::with_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorResponse {
                message: "Internal server error".to_string(),
            },
        )
    }

    fn invalid_method() -> ockam_core::Result<ControlApiHttpResponse> {
        Self::with_body(
            StatusCode::METHOD_NOT_ALLOWED,
            ErrorResponse {
                message: "Method not allowed".to_string(),
            },
        )
    }

    pub(crate) fn missing_resource_id() -> ockam_core::Result<ControlApiHttpResponse> {
        Self::with_body(
            StatusCode::BAD_REQUEST,
            ErrorResponse {
                message: "Missing resource ID".to_string(),
            },
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    message: String,
}

fn build_error_body(message: &str) -> Full<Bytes> {
    let result = serde_json::to_vec(&ErrorResponse {
        message: message.to_string(),
    });

    match result {
        Ok(body) => Full::new(Bytes::from(body)),
        Err(error) => {
            error!("Failed to encode error response body: {error:?}");
            Full::new(Bytes::from("{\"message\": \"Internal server error\"}"))
        }
    }
}
