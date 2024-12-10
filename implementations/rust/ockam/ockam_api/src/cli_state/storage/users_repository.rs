use crate::cloud::email_address::EmailAddress;
use crate::cloud::enroll::auth0::UserInfo;
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::retry;

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

#[async_trait]
impl<T: UsersRepository> UsersRepository for AutoRetry<T> {
    async fn store_user(&self, user: &UserInfo) -> ockam_core::Result<()> {
        retry!(self.wrapped.store_user(user))
    }

    async fn get_default_user(&self) -> ockam_core::Result<Option<UserInfo>> {
        retry!(self.wrapped.get_default_user())
    }

    async fn set_default_user(&self, email: &EmailAddress) -> ockam_core::Result<()> {
        retry!(self.wrapped.set_default_user(email))
    }

    async fn get_user(&self, email: &EmailAddress) -> ockam_core::Result<Option<UserInfo>> {
        retry!(self.wrapped.get_user(email))
    }

    async fn get_users(&self) -> ockam_core::Result<Vec<UserInfo>> {
        retry!(self.wrapped.get_users())
    }

    async fn delete_user(&self, email: &EmailAddress) -> ockam_core::Result<()> {
        retry!(self.wrapped.delete_user(email))
    }
}
