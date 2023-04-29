use ockam_identity::credential::OneTimeCode;
use serde::{Deserialize, Serialize};

use crate::config::{cli::TrustContextConfig, lookup::ProjectLookup};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnrollmentTicket {
    one_time_code: OneTimeCode,
    project: Option<ProjectLookup>,
    trust_context: Option<TrustContextConfig>,
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

    pub fn one_time_code(&self) -> &OneTimeCode {
        &self.one_time_code
    }

    pub fn project(&self) -> Option<&ProjectLookup> {
        self.project.as_ref()
    }
}
