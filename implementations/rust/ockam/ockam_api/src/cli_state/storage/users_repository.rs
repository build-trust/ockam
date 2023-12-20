use crate::cloud::email_address::EmailAddress;
use crate::cloud::enroll::auth0::UserInfo;
use ockam_core::async_trait;
use ockam_core::Result;

/// This traits allows user information to be stored locally.
/// User information is retrieved when a user has been authenticated.
/// It contains fields like:
///
///  - name
///  - sub(ject) unique identifier
///  - email
///  - etc...
///
/// Even if there is a sub field supposed to uniquely identify a user we currently use
/// the user email for this.
///
/// A user can also be set as the default user via this repository.
///
#[async_trait]
pub trait UsersRepository: Send + Sync + 'static {
    /// Store (or update) some information
    /// In case of an update, if the user was already the default user, it will stay the default user
    async fn store_user(&self, user: &UserInfo) -> Result<()>;

    /// Return the default user
    async fn get_default_user(&self) -> Result<Option<UserInfo>>;

    /// Set a user as the default one
    async fn set_default_user(&self, email: &EmailAddress) -> Result<()>;

    /// Return a user given their email
    async fn get_user(&self, email: &EmailAddress) -> Result<Option<UserInfo>>;

    /// Get the list of all users
    async fn get_users(&self) -> Result<Vec<UserInfo>>;

    /// Delete a user given their email
    async fn delete_user(&self, email: &EmailAddress) -> Result<()>;
}
