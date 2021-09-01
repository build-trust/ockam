use rand::random;
use reqwest::header::{HeaderMap, HeaderValue};
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Represents potential InfluxDB errors. Specifically, we are interested in categorizing authentication
/// errors distinctly from other errors. This allows us to take specific actions, such as revoking a lease.
#[derive(Debug, Clone)]
pub enum InfluxError {
    Authentication,
    Unknown,
}

impl Display for InfluxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Something went wrong.")
    }
}

impl Error for InfluxError {}

impl InfluxError {
    pub fn is_authentication_error(&self) -> bool {
        matches!(self, InfluxError::Authentication)
    }
}

/// A basic InfluxDB client. Contains InfluxDB meta-data and a leased token.
pub struct InfluxClient {
    api_url: String,
    org: String,
    bucket: String,
    leased_token: String,
}

impl InfluxClient {
    /// Create a new client.
    pub fn new(api_url: &str, org: &str, bucket: &str, leased_token: &str) -> Self {
        InfluxClient {
            api_url: api_url.to_string(),
            org: org.to_string(),
            bucket: bucket.to_string(),
            leased_token: leased_token.to_string(),
        }
    }

    /// Set the current token.
    pub fn set_token(&mut self, leased_token: &str) {
        self.leased_token = leased_token.to_string();
    }

    /// Send some random metrics to InfluxDB.
    pub async fn send_metrics(&self) -> Result<(), InfluxError> {
        let url = format!(
            "{}/api/v2/write?org={}&bucket={}&precision=s",
            self.api_url, self.org, self.bucket
        );

        let mut headers = HeaderMap::new();
        let token = format!("Token {}", self.leased_token);

        headers.insert(
            "Authorization",
            HeaderValue::from_str(token.as_str()).unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        // Send 10 data points. On authentication error (403), return an `InfluxError::Authentication`
        for i in 0..10 {
            let data = random::<usize>() % 10_000;
            let metric = format!("metrics,env=test r{}={}", i, data);
            let resp = client.post(url.clone()).body(metric).send().await.unwrap();
            let status = resp.status().as_u16();
            if (401..=403).contains(&status) {
                return Err(InfluxError::Authentication);
            }
        }
        Ok(())
    }
}
