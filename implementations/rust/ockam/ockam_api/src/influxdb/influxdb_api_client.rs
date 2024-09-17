use crate::influxdb::lease_token::{LeaseToken, TokenStatus};
use crate::ApiError;
use ockam::identity::Identifier;
use ockam_core::api::{Response, Status};
use ockam_core::async_trait;
use reqwest::Client;
use std::str::FromStr;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[async_trait]
pub trait InfluxDBApi {
    async fn create_token(
        &self,
        req: InfluxDBCreateTokenRequest,
    ) -> ockam_core::Result<InfluxDBResponse<InfluxDBTokenResponse>>;

    async fn get_token(
        &self,
        token_id: &str,
    ) -> ockam_core::Result<InfluxDBResponse<InfluxDBTokenResponse>>;

    async fn revoke_token(&self, token_id: &str) -> ockam_core::Result<InfluxDBEmptyResponse>;

    async fn list_tokens(&self)
        -> ockam_core::Result<InfluxDBResponse<InfluxDBListTokensResponse>>;
}

#[derive(Debug, Clone)]
pub struct InfluxDBApiClient {
    http_client: Client,
    base_url: String,
    token: String,
}

impl InfluxDBApiClient {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> ockam_core::Result<Self> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        debug!(%base_url, "Creating InfluxDBApiClient");
        let http_client = reqwest::ClientBuilder::new()
            .build()
            .map_err(|e| ApiError::core(format!("Failed to create http client: {e}")))?;
        Ok(Self {
            http_client,
            base_url,
            token: token.into(),
        })
    }

    async fn parse_response<T>(res: reqwest::Response) -> ockam_core::Result<InfluxDBResponse<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let bytes = res.bytes().await.map_err(|e| {
            error!("Failed to read InfluxDB response: {e:?}");
            ApiError::core(format!("Failed to read InfluxDB response: {e:?}"))
        })?;
        match serde_json::from_slice::<InfluxDBResponse<T>>(&bytes) {
            Ok(res) => Ok(res),
            Err(e) => {
                let text = String::from_utf8_lossy(&bytes);
                error!("Failed to parse InfluxDB response {text}/n with err {e:?}");
                Err(ApiError::core(format!(
                    "Failed to parse InfluxDB response {text}/n with err {e:?}"
                )))
            }
        }
    }

    async fn parse_empty_response(
        res: reqwest::Response,
    ) -> ockam_core::Result<InfluxDBEmptyResponse> {
        if res.status().is_success() {
            Ok(InfluxDBEmptyResponse {
                code: None,
                message: None,
            })
        } else {
            match res.json::<InfluxDBEmptyResponse>().await {
                Ok(res) => Ok(res),
                Err(e) => {
                    error!("Failed to parse InfluxDB response: {e:?}");
                    Err(ApiError::core(format!(
                        "Failed to parse InfluxDB response: {e:?}"
                    )))
                }
            }
        }
    }
}

#[async_trait]
impl InfluxDBApi for InfluxDBApiClient {
    async fn create_token(
        &self,
        req: InfluxDBCreateTokenRequest,
    ) -> ockam_core::Result<InfluxDBResponse<InfluxDBTokenResponse>> {
        let req = self
            .http_client
            .post(format!("{}/api/v2/authorizations", self.base_url))
            .header("Authorization", format!("Token {}", self.token))
            .header("Content-Type", "application/json")
            .body(format!(
                "{{\"description\": \"{}\", \"orgID\": \"{}\", \"permissions\":{}}}",
                req.description, req.org_id, req.permissions
            ));
        match req.send().await {
            Ok(res) => Self::parse_response::<InfluxDBTokenResponse>(res).await,
            Err(e) => {
                error!("Failed to create token: {e:?}");
                Err(ApiError::core(format!("Failed to create token: {e:?}")))
            }
        }
    }

    async fn get_token(
        &self,
        token_id: &str,
    ) -> ockam_core::Result<InfluxDBResponse<InfluxDBTokenResponse>> {
        let req = self
            .http_client
            .get(format!(
                "{}/api/v2/authorizations/{}",
                self.base_url, token_id
            ))
            .header("Authorization", format!("Token {}", self.token))
            .header("Content-Type", "application/json");
        match req.send().await {
            Ok(res) => Self::parse_response::<InfluxDBTokenResponse>(res).await,
            Err(e) => {
                error!("Failed to create token: {e:?}");
                Err(ApiError::core(format!("Failed to create token: {e:?}")))
            }
        }
    }

    async fn revoke_token(&self, token_id: &str) -> ockam_core::Result<InfluxDBEmptyResponse> {
        let req = self
            .http_client
            .delete(format!(
                "{}/api/v2/authorizations/{}",
                self.base_url, token_id
            ))
            .header("Authorization", format!("Token {}", self.token))
            .header("Content-Type", "application/json");
        match req.send().await {
            Ok(res) => Self::parse_empty_response(res).await,
            Err(e) => {
                error!("Failed to revoke token: {e:?}");
                Err(ApiError::core(format!("Failed to revoke token: {e:?}")))
            }
        }
    }

    async fn list_tokens(
        &self,
    ) -> ockam_core::Result<InfluxDBResponse<InfluxDBListTokensResponse>> {
        let req = self
            .http_client
            .get(format!("{}/api/v2/authorizations", self.base_url))
            .header("Authorization", format!("Token {}", self.token))
            .header("Content-Type", "application/json");
        match req.send().await {
            Ok(res) => Self::parse_response::<InfluxDBListTokensResponse>(res).await,
            Err(e) => {
                error!("Failed to list tokens: {e:?}");
                Err(ApiError::core(format!("Failed to list tokens: {e:?}")))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct InfluxDBCreateTokenRequest {
    pub description: String,
    pub org_id: String,
    pub permissions: String,
}

impl InfluxDBCreateTokenRequest {
    pub fn new(
        org_id: String,
        permissions: String,
        requester: &Identifier,
        expires: OffsetDateTime,
    ) -> Self {
        Self {
            description: InfluxDBCreateTokenRequest::pack_metadata(requester, expires),
            org_id,
            permissions,
        }
    }

    /// The InfluxDB tokens only have a description field that can be used to store metadata.
    /// The Ockam `LeaseToken` will pack in the description field the identifier that created the token,
    /// and its expiration time.
    pub fn pack_metadata(requester: &Identifier, expires: OffsetDateTime) -> String {
        format!("OCKAM:{}:{}", requester, expires.unix_timestamp()).to_string()
    }
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
pub struct InfluxDBResponse<T> {
    pub code: Option<InfluxDBResponseCode>,
    pub message: Option<String>,
    #[serde(flatten)]
    pub data: Option<T>,
}

impl<T> InfluxDBResponse<T> {
    pub fn status(&self) -> Status {
        match &self.code {
            Some(code) => code.to_status(),
            None => Status::Ok,
        }
    }

    pub fn into_response(self) -> Result<T, Response<ockam_core::api::Error>> {
        if let Some(data) = self.data {
            Ok(data)
        } else {
            let status = self.status();
            let message = self
                .message
                .unwrap_or_else(|| "Failed to parse influxdb api response".to_string());
            error!(%status, %message, "InfluxDB request returned error");
            let err = ockam_core::api::Error::new_without_path().with_message(message);
            Err(Response::with_status_no_request(status).body(err))
        }
    }
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
pub struct InfluxDBEmptyResponse {
    pub code: Option<InfluxDBResponseCode>,
    pub message: Option<String>,
}

impl InfluxDBEmptyResponse {
    pub fn status(&self) -> Status {
        match &self.code {
            Some(code) => code.to_status(),
            None => Status::Ok,
        }
    }

    pub fn into_response(self) -> Result<(), Response<ockam_core::api::Error>> {
        let status = self.status();
        if status.eq(&Status::Ok) {
            Ok(())
        } else {
            let message = self
                .message
                .unwrap_or_else(|| "Failed to parse influxdb api response".to_string());
            error!(%status, %message, "InfluxDB request returned error");
            let err = ockam_core::api::Error::new_without_path().with_message(message);
            Err(Response::with_status_no_request(status).body(err))
        }
    }
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
pub enum InfluxDBResponseCode {
    #[serde(rename = "invalid")]
    Invalid,
    #[serde(rename = "unauthorized")]
    Unauthorized,
    #[serde(rename = "forbidden")]
    Forbidden,
    #[serde(rename = "not found")]
    NotFound,
    #[serde(rename = "conflict")]
    Conflict,
    #[serde(rename = "internal error")]
    InternalError,
}

impl InfluxDBResponseCode {
    pub fn to_status(&self) -> Status {
        match self {
            Self::Invalid => Status::BadRequest,
            Self::Unauthorized => Status::Unauthorized,
            Self::Forbidden => Status::Forbidden,
            Self::NotFound => Status::NotFound,
            Self::Conflict => Status::Conflict,
            Self::InternalError => Status::InternalServerError,
        }
    }
}

/// Token returned by InfluxDB API
#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
pub struct InfluxDBTokenResponse {
    pub id: String,
    pub description: String,
    pub token: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

/// Return a `LeaseToken` if it's an Ockam token (i.e., if the `description` contains a valid Ockam metadata).
/// If the metadata is not found, the token will be ignored.
impl TryFrom<InfluxDBTokenResponse> for Option<LeaseToken> {
    type Error = ockam_core::Error;

    fn try_from(token: InfluxDBTokenResponse) -> Result<Self, Self::Error> {
        match token.unpack_metadata()? {
            Some((issued_for, expires)) => Ok(Some(LeaseToken {
                id: token.id,
                issued_for,
                created_at: OffsetDateTime::parse(&token.created_at, &Rfc3339)
                    .map_err(|_| {
                        ApiError::core(format!(
                            "Expected Rfc3339 format for 'created_at' with value {}",
                            token.created_at
                        ))
                    })?
                    .unix_timestamp(),
                expires_at: expires.unix_timestamp(),
                status: TokenStatus::from_str(&token.status)?,
                token: token.token,
            })),
            None => Ok(None),
        }
    }
}

impl InfluxDBTokenResponse {
    /// Unpack the metadata from the description field.
    pub fn unpack_metadata(&self) -> ockam_core::Result<Option<(Identifier, OffsetDateTime)>> {
        let segments = self.description.split(':').collect::<Vec<_>>();
        match segments[..] {
            ["OCKAM", identifier, expires] => {
                let identifier = Identifier::try_from(identifier)?;
                let expires_timestamp: i64 = expires
                    .parse()
                    .map_err(|_| ApiError::core("Invalid 'expires' timestamp"))?;
                let expires = OffsetDateTime::from_unix_timestamp(expires_timestamp)
                    .map_err(|_| ApiError::core("Invalid 'expires' timestamp"))?;
                Ok(Some((identifier, expires)))
            }
            _ => Ok(None),
        }
    }
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
pub struct InfluxDBListTokensResponse {
    #[serde(rename = "authorizations")]
    pub tokens: Vec<InfluxDBTokenResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::influxdb::lease_token::{LeaseToken, TokenStatus};
    use std::str::FromStr;
    use time::OffsetDateTime;

    #[test]
    fn lease_token_from_influxdb_token() {
        let identifier = "I0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let expires_at = OffsetDateTime::now_utc() + core::time::Duration::from_secs(60);
        let expires_at_timestamp = expires_at.unix_timestamp();
        let created_at = "2024-09-12T16:23:54Z";
        let created_at_timestamp = 1726158234;
        let token = InfluxDBTokenResponse {
            id: "token_id".to_string(),
            description: format!("OCKAM:{identifier}:{expires_at_timestamp}"),
            token: "token".to_string(),
            status: "active".to_string(),
            created_at: created_at.to_string(),
        };
        let expected = LeaseToken {
            id: "token_id".to_string(),
            issued_for: Identifier::from_str(identifier).unwrap(),
            created_at: created_at_timestamp,
            expires_at: expires_at_timestamp,
            token: "token".to_string(),
            status: TokenStatus::Active,
        };
        let got = {
            let got: Option<LeaseToken> = token.try_into().unwrap();
            got.unwrap()
        };
        assert_eq!(got, expected);
    }
}
