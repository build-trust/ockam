use ockam::identity::OneTimeCode;
use ockam_core::Result;
use serde::{Deserialize, Serialize};

use crate::config::{cli::TrustContextConfig, lookup::ProjectLookup};
use crate::error::ApiError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnrollmentTicket {
    pub one_time_code: OneTimeCode,
    pub project: Option<ProjectLookup>,
    pub trust_context: Option<TrustContextConfig>,
}

impl EnrollmentTicket {
    pub fn new(
        one_time_code: OneTimeCode,
        project: Option<ProjectLookup>,
        trust_context: Option<TrustContextConfig>,
    ) -> Self {
        Self {
            one_time_code,
            project,
            trust_context,
        }
    }

    pub fn hex_encoded(&self) -> Result<String> {
        let serialized = serde_json::to_vec(&self)
            .map_err(|_err| ApiError::core("Failed to authenticate with Okta"))?;
        Ok(hex::encode(serialized))
    }
}
