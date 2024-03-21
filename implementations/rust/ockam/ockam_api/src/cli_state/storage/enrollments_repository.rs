use ockam::identity::Identifier;
use ockam_core::async_trait;
use ockam_core::Result;

use crate::cli_state::enrollments::IdentityEnrollment;
use crate::cloud::email_address::EmailAddress;

/// This trait stores the enrollment status for local identities
/// If an identity has been enrolled it is possible to retrieve:
///
///  - its name (if it has one)
///  - if this the default identity
///  - if an identity was enrolled and when the local node was informed
///
///
#[async_trait]
pub trait EnrollmentsRepository: Send + Sync + 'static {
    /// Set the identifier as enrolled, and set a timestamp to record the information
    async fn set_as_enrolled(&self, identifier: &Identifier, email: &EmailAddress) -> Result<()>;

    /// Get the list of enrolled identities
    async fn get_enrolled_identities(&self) -> Result<Vec<IdentityEnrollment>>;

    /// Get the enrollment statuses for all known identities
    async fn get_all_identities_enrollments(&self) -> Result<Vec<IdentityEnrollment>>;

    /// Return true if the default identity is enrolled
    async fn is_default_identity_enrolled(&self) -> Result<bool>;

    /// Return true if the identity with the given name is enrolled
    async fn is_identity_enrolled(&self, name: &str) -> Result<bool>;
}
