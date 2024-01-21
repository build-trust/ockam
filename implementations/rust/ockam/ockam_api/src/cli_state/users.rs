use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

use crate::cli_state::CliState;
use crate::cli_state::Result;
use crate::cloud::email_address::EmailAddress;
use crate::cloud::enroll::auth0::UserInfo;

impl CliState {
    pub async fn store_user(&self, user: &UserInfo) -> Result<()> {
        let repository = self.users_repository().await?;
        let is_first_user = repository.get_users().await?.is_empty();
        repository.store_user(user).await?;

        // if this is the first user we store we mark it as the default user
        if is_first_user {
            self.set_default_user(&user.email).await?
        }
        Ok(())
    }

    pub async fn set_default_user(&self, email: &EmailAddress) -> Result<()> {
        self.users_repository()
            .await?
            .set_default_user(email)
            .await?;
        Ok(())
    }

    pub async fn get_default_user(&self) -> Result<UserInfo> {
        let repository = self.users_repository().await?;
        match repository.get_default_user().await? {
            Some(user) => Ok(user),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "there is no default user",
            ))?,
        }
    }
}
