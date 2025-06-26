use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use tracing::Level;

use crate::cli_state::CliState;
use crate::cli_state::Result;
use crate::orchestrator::email_address::EmailAddress;
use crate::orchestrator::enroll::auth0::UserInfo;

impl CliState {
    #[instrument(skip_all, fields(user = %user), level = Level::TRACE)]
    pub async fn store_user(&self, user: &UserInfo) -> Result<()> {
        let repository = self.users_repository();
        repository.store_user(user).await?;
        Ok(())
    }

    #[instrument(skip_all, fields(email = %email), level = Level::TRACE)]
    pub async fn set_default_user(&self, email: &EmailAddress) -> Result<()> {
        self.users_repository().set_default_user(email).await?;
        Ok(())
    }

    #[instrument(skip_all, level = Level::TRACE)]
    pub async fn get_default_user(&self) -> Result<UserInfo> {
        let repository = self.users_repository();
        match repository.get_default_user().await? {
            Some(user) => Ok(user),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "there is no default user",
            ))?,
        }
    }

    #[instrument(skip_all, level = Level::TRACE)]
    pub async fn get_user(&self, email: &EmailAddress) -> Result<UserInfo> {
        let repository = self.users_repository();
        match repository.get_user(email).await? {
            Some(user) => Ok(user),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no user with email {email}"),
            ))?,
        }
    }
}
