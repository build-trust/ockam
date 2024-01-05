use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

use crate::cli_state::CliState;
use crate::cloud::space::Space;

use super::Result;

impl CliState {
    pub async fn store_space(
        &self,
        space_id: &str,
        space_name: &str,
        users: Vec<&str>,
    ) -> Result<Space> {
        let repository = self.spaces_repository().await?;
        let space = Space {
            id: space_id.to_string(),
            name: space_name.to_string(),
            users: users.iter().map(|u| u.to_string()).collect(),
        };

        repository.store_space(&space).await?;

        // If there is no previous default space set this space as the default
        let default_space = repository.get_default_space().await?;
        if default_space.is_none() {
            repository.set_default_space(&space.id).await?
        };

        Ok(space)
    }

    pub async fn get_default_space(&self) -> Result<Space> {
        match self.spaces_repository().await?.get_default_space().await? {
            Some(space) => Ok(space),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                "there is no default space",
            ))?,
        }
    }

    pub async fn get_space_by_name(&self, name: &str) -> Result<Space> {
        match self
            .spaces_repository()
            .await?
            .get_space_by_name(name)
            .await?
        {
            Some(space) => Ok(space),
            None => Err(Error::new(
                Origin::Api,
                Kind::NotFound,
                format!("there is no space with name {name}"),
            ))?,
        }
    }

    pub async fn get_spaces(&self) -> Result<Vec<Space>> {
        Ok(self.spaces_repository().await?.get_spaces().await?)
    }

    pub async fn delete_space(&self, space_id: &str) -> Result<()> {
        let repository = self.spaces_repository().await?;
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

    pub async fn set_space_as_default(&self, space_id: &str) -> Result<()> {
        Ok(self
            .spaces_repository()
            .await?
            .set_default_space(space_id)
            .await?)
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
            .store_space("1", "name1", vec!["me@ockam.io", "you@ockam.io"])
            .await?;
        let result = cli.get_default_space().await?;
        assert_eq!(result, space1);

        // the store method can be used to update a space
        let updated_space1 = cli.store_space("1", "name1", vec!["them@ockam.io"]).await?;
        let result = cli.get_default_space().await?;
        assert_eq!(result, updated_space1);

        Ok(())
    }
}
