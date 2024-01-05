use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use ockam::identity::Identifier;
use ockam::identity::OneTimeCode;

use crate::cli_state::Result;
use crate::cli_state::{CliState, CliStateError};
use crate::cloud::project::Project;
use crate::error::ApiError;

/// The following CliState methods help keeping track of
///
impl CliState {
    pub async fn is_identity_enrolled(&self, name: &Option<String>) -> Result<bool> {
        let repository = self.enrollment_repository().await?;

        match name {
            Some(name) => Ok(repository.is_identity_enrolled(name).await?),
            None => Ok(repository.is_default_identity_enrolled().await?),
        }
    }

    pub async fn is_default_identity_enrolled(&self) -> Result<bool> {
        Ok(self
            .enrollment_repository()
            .await?
            .is_default_identity_enrolled()
            .await?)
    }

    pub async fn set_identifier_as_enrolled(&self, identifier: &Identifier) -> Result<()> {
        Ok(self
            .enrollment_repository()
            .await?
            .set_as_enrolled(identifier)
            .await?)
    }

    /// Return information of enrolled entities. Either:
    ///
    ///  - all the currently enrolled entities
    ///  - all the known identities and their corresponding enrollment state
    pub async fn get_identity_enrollments(
        &self,
        enrollment_status: EnrollmentStatus,
    ) -> Result<Vec<IdentityEnrollment>> {
        let repository = self.enrollment_repository().await?;
        match enrollment_status {
            EnrollmentStatus::Enrolled => Ok(repository.get_enrolled_identities().await?),
            EnrollmentStatus::Any => Ok(repository.get_all_identities_enrollments().await?),
        }
    }

    /// Return true if the user is enrolled.
    /// At the moment this check only verifies that there is a default project.
    /// This project should be the project that is created at the end of the enrollment procedure
    pub async fn is_enrolled(&self) -> miette::Result<bool> {
        if !self.is_default_identity_enrolled().await? {
            return Ok(false);
        }

        let default_space_exists = self.get_default_space().await.is_ok();
        if !default_space_exists {
            let message =
                "There should be a default space set for the current user. Please re-enroll";
            error!("{}", message);
            return Err(CliStateError::from(message))?;
        }

        let default_project_exists = self.get_default_project().await.is_ok();
        if !default_project_exists {
            let message =
                "There should be a default project set for the current user. Please re-enroll";
            error!("{}", message);
            return Err(CliStateError::from(message))?;
        }

        Ok(true)
    }
}

pub enum EnrollmentStatus {
    Enrolled,
    Any,
}

pub struct IdentityEnrollment {
    identifier: Identifier,
    name: Option<String>,
    is_default: bool,
    enrolled_at: Option<OffsetDateTime>,
}

impl IdentityEnrollment {
    pub fn new(
        identifier: Identifier,
        name: Option<String>,
        is_default: bool,
        enrolled_at: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            identifier,
            name,
            is_default,
            enrolled_at,
        }
    }
    pub fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }

    #[allow(dead_code)]
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    #[allow(dead_code)]
    pub fn is_enrolled(&self) -> bool {
        self.enrolled_at.is_some()
    }

    #[allow(dead_code)]
    pub fn is_default(&self) -> bool {
        self.is_default
    }

    #[allow(dead_code)]
    pub fn enrolled_at(&self) -> Option<OffsetDateTime> {
        self.enrolled_at
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnrollmentTicket {
    pub one_time_code: OneTimeCode,
    pub project: Option<Project>,
}

impl EnrollmentTicket {
    pub fn new(one_time_code: OneTimeCode, project: Option<Project>) -> Self {
        Self {
            one_time_code,
            project,
        }
    }

    pub fn hex_encoded(&self) -> Result<String> {
        let serialized = serde_json::to_vec(&self)
            .map_err(|_err| ApiError::core("Failed to authenticate with Okta"))?;
        Ok(hex::encode(serialized))
    }
}
