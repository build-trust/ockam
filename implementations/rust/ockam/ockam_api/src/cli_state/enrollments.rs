use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use time::OffsetDateTime;

use crate::authenticator::one_time_code::OneTimeCode;
use ockam::identity::Identifier;

use crate::cli_state::Result;
use crate::cli_state::{CliState, CliStateError};
use crate::cloud::email_address::EmailAddress;
use crate::cloud::project::models::ProjectModel;
use crate::error::ApiError;

/// The following CliState methods help keeping track of
///
impl CliState {
    #[instrument(skip_all, fields(name = name.clone()))]
    pub async fn is_identity_enrolled(&self, name: &Option<String>) -> Result<bool> {
        let repository = self.enrollment_repository();

        match name {
            Some(name) => Ok(repository.is_identity_enrolled(name).await?),
            None => Ok(repository.is_default_identity_enrolled().await?),
        }
    }

    #[instrument(skip_all)]
    pub async fn is_default_identity_enrolled(&self) -> Result<bool> {
        Ok(self
            .enrollment_repository()
            .is_default_identity_enrolled()
            .await?)
    }

    #[instrument(skip_all)]
    pub async fn identity_should_enroll(&self, name: &Option<String>, force: bool) -> Result<bool> {
        if force {
            return Ok(true);
        }

        // Force enrollment if there are no spaces or projects in the database
        if self.get_spaces().await?.is_empty() || self.projects().get_projects().await?.is_empty() {
            return Ok(true);
        }

        // Force enrollment if the identity is not enrolled
        Ok(!self.is_identity_enrolled(name).await?)
    }

    #[instrument(skip_all, fields(identifier = %identifier))]
    pub async fn set_identifier_as_enrolled(
        &self,
        identifier: &Identifier,
        email: &EmailAddress,
    ) -> Result<()> {
        Ok(self
            .enrollment_repository()
            .set_as_enrolled(identifier, email)
            .await?)
    }

    /// Return information of enrolled entities. Either:
    ///
    ///  - all the currently enrolled entities
    ///  - all the known identities and their corresponding enrollment state
    #[instrument(skip_all, fields(enrollment_status = %enrollment_status))]
    pub async fn get_identity_enrollments(
        &self,
        enrollment_status: EnrollmentStatus,
    ) -> Result<Vec<IdentityEnrollment>> {
        let repository = self.enrollment_repository();
        match enrollment_status {
            EnrollmentStatus::Enrolled => Ok(repository.get_enrolled_identities().await?),
            EnrollmentStatus::Any => Ok(repository.get_all_identities_enrollments().await?),
        }
    }

    /// Return true if the user is enrolled.
    /// At the moment this check only verifies that there is a default project.
    /// This project should be the project that is created at the end of the enrollment procedure
    #[instrument(skip_all)]
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

        let default_project_exists = self.projects().get_default_project().await.is_ok();
        if !default_project_exists {
            let message =
                "There should be a default project set for the current user. Please re-enroll";
            error!("{}", message);
            return Err(CliStateError::from(message))?;
        }

        Ok(true)
    }
}

#[derive(Debug)]
pub enum EnrollmentStatus {
    Enrolled,
    Any,
}

impl Display for EnrollmentStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EnrollmentStatus::Enrolled => f.write_str("enrolled"),
            EnrollmentStatus::Any => f.write_str("any"),
        }
    }
}

pub struct IdentityEnrollment {
    identifier: Identifier,
    name: Option<String>,
    email: Option<EmailAddress>,
    is_default: bool,
    enrolled_at: Option<OffsetDateTime>,
}

impl IdentityEnrollment {
    pub fn new(
        identifier: Identifier,
        name: Option<String>,
        email: Option<EmailAddress>,
        is_default: bool,
        enrolled_at: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            identifier,
            name,
            email,
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
    pub fn email(&self) -> Option<EmailAddress> {
        self.email.clone()
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnrollmentTicket {
    pub one_time_code: OneTimeCode,
    pub project: Option<ProjectModel>,
}

impl EnrollmentTicket {
    pub fn new(one_time_code: OneTimeCode, project: Option<ProjectModel>) -> Self {
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

#[cfg(test)]
mod tests {}
