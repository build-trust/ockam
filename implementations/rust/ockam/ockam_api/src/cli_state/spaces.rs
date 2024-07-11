use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

use crate::cli_state::CliState;
use crate::cloud::space::Space;
use crate::cloud::subscription::Subscription;

use super::Result;

impl CliState {
    #[instrument(skip_all, fields(space_id = space_id, space_name = space_name))]
    pub async fn store_space(
        &self,
        space_id: &str,
        space_name: &str,
        users: Vec<&str>,
        subscription: Option<&Subscription>,
    ) -> Result<Space> {
        let repository = self.spaces_repository();
        let space = Space {
            id: space_id.to_string(),
            name: space_name.to_string(),
            users: users.iter().map(|u| u.to_string()).collect(),
            subscription: subscription.cloned(),
        };

        repository.store_space(&space).await?;

        // If there is no previous default space set this space as the default
        let default_space = repository.get_default_space().await?;
        if default_space.is_none() {
            repository.set_default_space(&space.id).await?
        };

        Ok(space)
    }

    #[instrument(skip_all)]
    pub async fn get_default_space(&self) -> Result<Space> {
        match self.spaces_repository().get_default_space().await? {
            Some(space) => Ok(space),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "there is no default space",
            ))?,
        }
    }

    #[instrument(skip_all, fields(name = name))]
    pub async fn get_space_by_name(&self, name: &str) -> Result<Space> {
        match self.spaces_repository().get_space_by_name(name).await? {
            Some(space) => Ok(space),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no space with name {name}"),
            ))?,
        }
    }

    #[instrument(skip_all, fields(name = name))]
    pub async fn get_space_by_name_or_default(&self, name: &Option<String>) -> Result<Space> {
        match name {
            Some(name) => self.get_space_by_name(name.as_str()).await,
            None => self.get_default_space().await,
        }
    }

    #[instrument(skip_all)]
    pub async fn get_spaces(&self) -> Result<Vec<Space>> {
        Ok(self.spaces_repository().get_spaces().await?)
    }

    #[instrument(skip_all, fields(space_id = space_id))]
    pub async fn delete_space(&self, space_id: &str) -> Result<()> {
        let repository = self.spaces_repository();
        // delete the space
        let space_exists = repository.get_space(space_id).await.is_ok();
        repository.delete_space(space_id).await?;

        // set another space as the default space
        if space_exists {
            let other_space = repository.get_spaces().await?;
            if let Some(other_space) = other_space.first() {
                repository
                    .set_default_space(&other_space.space_id())
                    .await?;
            }
        }
        Ok(())
    }

    #[instrument(skip_all, fields(space_id = space_id))]
    pub async fn set_space_as_default(&self, space_id: &str) -> Result<()> {
        Ok(self.spaces_repository().set_default_space(space_id).await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_cli_spaces() -> Result<()> {
        let cli = CliState::test().await?;

        // the first created space becomes the default
        let space1 = cli
            .store_space(
                "1",
                "name1",
                vec!["me@ockam.io", "you@ockam.io"],
                Some(&Subscription::new(
                    "name1".to_string(),
                    false,
                    None,
                    None,
                    None,
                )),
            )
            .await?;
        let result = cli.get_default_space().await?;
        assert_eq!(result, space1);

        // the store method can be used to update a space
        let updated_space1 = cli
            .store_space("1", "name1", vec!["them@ockam.io"], None)
            .await?;
        let result = cli.get_default_space().await?;
        assert_eq!(result, updated_space1);

        Ok(())
    }
}
