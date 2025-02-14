use crate::control_api::protocol::common::ErrorResponse;
use crate::control_api::ControlApiError;
use bytes::Bytes;
use http::StatusCode;
use http_body_util::Full;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use serde::Serialize;

#[derive(Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct ControlApiHttpRequest {
    #[n(0)] pub method: String,
    #[n(1)] pub uri: String,
    #[n(2)] pub body: Option<Vec<u8>>,
}

#[derive(Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct ControlApiHttpResponse {
    #[n(0)] pub status: u16,
    #[n(1)] pub body: Vec<u8>,
}

impl ControlApiHttpResponse {
    pub fn with_body<T: Serialize>(
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

    pub fn without_body(status_code: StatusCode) -> ockam_core::Result<ControlApiHttpResponse> {
        Ok(Self {
            status: status_code.as_u16(),
            body: Vec::new(),
        })
    }

    pub fn invalid_body<T>() -> Result<T, ControlApiError> {
        Err(Self::with_body(
            StatusCode::BAD_REQUEST,
            ErrorResponse {
                message: "Invalid request body".to_string(),
            },
        )?
        .into())
    }

    pub fn missing_body<T>() -> Result<T, ControlApiError> {
        Err(Self::with_body(
            StatusCode::BAD_REQUEST,
            ErrorResponse {
                message: "Missing request body".to_string(),
            },
        )?
        .into())
    }

    pub fn bad_request<T>(message: &str) -> Result<T, ControlApiError> {
        Err(Self::with_body(
            StatusCode::BAD_REQUEST,
            ErrorResponse {
                message: format!("Bad request: {message}"),
            },
        )?
        .into())
    }

    pub fn not_found<T>(message: &str) -> Result<T, ControlApiError> {
        Err(Self::with_body(
            StatusCode::NOT_FOUND,
            ErrorResponse {
                message: message.to_string(),
            },
        )?
        .into())
    }

    pub fn internal_error<T>(error: &str) -> Result<T, ControlApiError> {
        Err(Self::with_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorResponse {
                message: format!("Internal server error: {error}"),
            },
        )?
        .into())
    }

    pub fn invalid_method<T>(
        method: &str,
        allowed_methods: Vec<&str>,
    ) -> Result<T, ControlApiError> {
        Err(Self::with_body(
            StatusCode::METHOD_NOT_ALLOWED,
            ErrorResponse {
                message: format!(
                    "Invalid method {method} for this API. Supported methods are: {}",
                    allowed_methods.join(", ")
                ),
            },
        )?
        .into())
    }

    pub fn missing_resource_id<T>(
        resource_name: &str,
        resource_name_identifier: &str,
    ) -> Result<T, ControlApiError> {
        Err(Self::with_body(
            StatusCode::BAD_REQUEST,
            ErrorResponse {
                message: format!("Missing parameter {resource_name}. The HTTP path should be /{{node-name}}/{resource_name}/{{{resource_name_identifier}}}"),
            },
        )?
        .into())
    }
}

pub fn build_error_body(message: &str) -> Full<Bytes> {
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
