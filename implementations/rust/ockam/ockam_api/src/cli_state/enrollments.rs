use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use time::OffsetDateTime;

use crate::authenticator::one_time_code::OneTimeCode;
use ockam::identity::{Identifier, TimestampInSeconds};

use crate::cli_state::Result;
use crate::cli_state::{CliState, CliStateError};
use crate::cloud::email_address::EmailAddress;
use crate::cloud::project::models::ProjectModel;
use crate::colors::{color_ok, color_primary, color_warn};
use crate::error::ApiError;
use crate::output::human_readable_time;
use crate::terminal::fmt;

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
    #[instrument(skip_all, fields(filter = %filter))]
    pub async fn get_identity_enrollments(
        &self,
        filter: EnrollmentFilter,
    ) -> Result<Vec<IdentityEnrollment>> {
        let repository = self.enrollment_repository();
        match filter {
            EnrollmentFilter::Enrolled => Ok(repository.get_enrolled_identities().await?),
            EnrollmentFilter::Any => Ok(repository.get_all_identities_enrollments().await?),
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
pub enum EnrollmentFilter {
    Enrolled,
    Any,
}

impl Display for EnrollmentFilter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EnrollmentFilter::Enrolled => f.write_str("enrolled"),
            EnrollmentFilter::Any => f.write_str("any"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum EnrollmentStatus {
    Enrolled {
        at: TimestampInSeconds,
        email: Option<EmailAddress>,
    },
    NotEnrolled,
}

impl EnrollmentStatus {
    pub fn is_enrolled(&self) -> bool {
        matches!(self, EnrollmentStatus::Enrolled { .. })
    }

    pub fn email(&self) -> Option<&EmailAddress> {
        match self {
            EnrollmentStatus::Enrolled { email, .. } => email.as_ref(),
            EnrollmentStatus::NotEnrolled => None,
        }
    }
}

#[derive(Serialize)]
pub struct IdentityEnrollment {
    identifier: Identifier,
    name: String,
    is_default: bool,
    status: EnrollmentStatus,
}

impl IdentityEnrollment {
    pub fn new(
        identifier: Identifier,
        name: String,
        is_default: bool,
        enrolled_at: Option<OffsetDateTime>,
        email: Option<EmailAddress>,
    ) -> Self {
        let status = match enrolled_at {
            Some(enrolled_at) => EnrollmentStatus::Enrolled {
                at: TimestampInSeconds::from(enrolled_at.unix_timestamp() as u64),
                email,
            },
            None => EnrollmentStatus::NotEnrolled,
        };
        Self {
            identifier,
            name,
            is_default,
            status,
        }
    }
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_default(&self) -> bool {
        self.is_default
    }

    pub fn status(&self) -> &EnrollmentStatus {
        &self.status
    }
}

impl Display for IdentityEnrollment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", fmt::PADDING, color_primary(self.name()))?;
        if self.is_default {
            write!(f, " (default)")?;
        }
        writeln!(f, ":")?;

        writeln!(
            f,
            "{}{}With Identifier {}",
            fmt::PADDING,
            fmt::INDENTATION,
            color_primary(self.identifier().to_string())
        )?;

        match &self.status {
            EnrollmentStatus::Enrolled { at, email } => {
                write!(
                    f,
                    "{}{}Was {} at {}",
                    fmt::PADDING,
                    fmt::INDENTATION,
                    color_ok("enrolled"),
                    color_primary(human_readable_time(*at))
                )?;
                if let Some(email) = email {
                    writeln!(f, " with email {}", color_primary(email.to_string()))?;
                } else {
                    writeln!(f)?;
                }
            }
            EnrollmentStatus::NotEnrolled => {
                writeln!(
                    f,
                    "{}{}Is {}",
                    fmt::PADDING,
                    fmt::INDENTATION,
                    color_warn("not enrolled")
                )?;
            }
        }

        Ok(())
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
