use anyhow::anyhow;
use clap::{Args, Parser};
#[cfg(test)]
use fake::{Dummy, Fake, Faker};

use ockam::Context;

use crate::api;
use crate::api::{
    CloudApi, CreateSpaceRequestPayload, DeleteSpacePayload, ListSpacesPayload, ShowSpacePayload,
};

#[derive(Clone, Debug, Parser)]
pub struct SpaceCommand {
    #[clap(subcommand)]
    pub command: SpacesSubCommand,
    #[clap(long, short, parse(from_occurrences))]
    pub verbose: u8,
}

#[derive(Clone, Debug, Parser)]
pub enum SpacesSubCommand {
    /// Creates a new space.
    #[clap(display_order = 1000)]
    Create(SpaceCommandArgs),
    /// List all spaces.
    #[clap(display_order = 1001)]
    List(SpaceListCommandArgs),
    /// Shows a single space.
    #[clap(display_order = 1002)]
    Show(SpaceCommandArgs),
    /// Delete a space.
    #[clap(display_order = 1003)]
    Delete(SpaceCommandArgs),
}

#[derive(Clone, Debug, Args)]
#[cfg_attr(test, derive(Dummy))]
pub struct SpaceCommandArgs {
    /// Name of the project that the space belongs to.
    pub project_name: String,
    /// Name of the space.
    pub space_name: String,
}

#[derive(Clone, Debug, Args)]
#[cfg_attr(test, derive(Dummy))]
pub struct SpaceListCommandArgs {
    /// Name of the project that the space belongs to.
    pub project_name: String,
}

pub async fn run(args: SpaceCommand, mut ctx: Context) -> anyhow::Result<()> {
    let mut api_client = api::NodeCloudApi::from(&mut ctx);
    let res = match args.command {
        SpacesSubCommand::Create(arg) => create(arg, &mut api_client).await,
        SpacesSubCommand::List(arg) => list(arg, &mut api_client).await,
        SpacesSubCommand::Show(arg) => show(arg, &mut api_client).await,
        SpacesSubCommand::Delete(arg) => delete(arg, &mut api_client).await,
    };
    api_client.ctx.stop().await?;
    res
}

pub async fn create<T>(args: SpaceCommandArgs, api_client: &mut T) -> anyhow::Result<()>
where
    T: CloudApi<CreateSpaceRequestPayload>,
{
    match api_client
        .send::<()>(CreateSpaceRequestPayload::from(args))
        .await
    {
        Ok(_) => {
            println!("Space created successfully");
            Ok(())
        }
        Err(err) => {
            tracing::error!("{:?}", err);
            Err(anyhow!("Failed to create space"))
        }
    }
}

pub async fn list<T>(args: SpaceListCommandArgs, api_client: &mut T) -> anyhow::Result<()>
where
    T: CloudApi<ListSpacesPayload>,
{
    //TODO: return type should be a struct representing the list of items.
    match api_client.send::<()>(ListSpacesPayload::from(args)).await {
        Ok(_res) => {
            // TODO
            // if let Some(res) = res {
            // println!("{res}");
            // };
            Ok(())
        }
        Err(err) => {
            tracing::error!("{:?}", err);
            Err(anyhow!("Failed to retrieve spaces"))
        }
    }
}

pub async fn show<T>(args: SpaceCommandArgs, api_client: &mut T) -> anyhow::Result<()>
where
    T: CloudApi<ShowSpacePayload>,
{
    //TODO: return type should be a struct representing the retrieved item.
    match api_client.send::<()>(ShowSpacePayload::from(args)).await {
        Ok(_res) => {
            // TODO
            // if let Some(res) = res {
            // println!("{res}");
            // };
            Ok(())
        }
        Err(err) => {
            tracing::error!("{:?}", err);
            Err(anyhow!("Failed to retrieve space"))
        }
    }
}

pub async fn delete<T>(args: SpaceCommandArgs, api_client: &mut T) -> anyhow::Result<()>
where
    T: CloudApi<DeleteSpacePayload>,
{
    match api_client.send::<()>(DeleteSpacePayload::from(args)).await {
        Ok(_) => {
            println!("Space deleted successfully");
            Ok(())
        }
        Err(err) => {
            tracing::error!("{:?}", err);
            Err(anyhow!("Failed to delete space"))
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use mockall::predicate::*;

    use crate::api::MockCloudApi;

    use super::*;

    mod node_api {
        use api::tests::node_api::*;

        use super::*;

        #[ockam::test(crate = "ockam")]
        async fn can_send_payloads(ctx: &mut ockam::Context) -> ockam::Result<()> {
            let (mut node_api, worker_name) = setup_node_api(ctx).await?;

            let payload: CreateSpaceRequestPayload = Faker.fake();
            send_payload::<CreateSpaceRequestPayload>(&mut node_api, &worker_name, payload).await?;

            let payload: ShowSpacePayload = Faker.fake();
            send_payload::<ShowSpacePayload>(&mut node_api, &worker_name, payload).await?;

            let payload: ListSpacesPayload = Faker.fake();
            send_payload::<ListSpacesPayload>(&mut node_api, &worker_name, payload).await?;

            let payload: DeleteSpacePayload = Faker.fake();
            send_payload::<DeleteSpacePayload>(&mut node_api, &worker_name, payload).await?;

            ctx.stop().await?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn create_space__happy_path() -> anyhow::Result<()> {
        let command_args: SpaceCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(CreateSpaceRequestPayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Ok(Some(())));

        let res = create(command_args, &mut api_client).await?;
        // assert_eq!((), res); //TODO

        Ok(())
    }

    #[tokio::test]
    async fn create_space__err_if_api_client_fails() -> anyhow::Result<()> {
        let command_args: SpaceCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(CreateSpaceRequestPayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Err(ockam::OckamError::BareError.into()));

        let res = create(command_args, &mut api_client).await;
        assert!(res.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn show_space__happy_path() -> anyhow::Result<()> {
        let command_args: SpaceCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(ShowSpacePayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Ok(Some(())));

        let res = show(command_args, &mut api_client).await?;
        // assert_eq!((), res); //TODO

        Ok(())
    }

    #[tokio::test]
    async fn show_space__err_if_api_client_fails() -> anyhow::Result<()> {
        let command_args: SpaceCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(ShowSpacePayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Err(ockam::OckamError::BareError.into()));

        let res = show(command_args, &mut api_client).await;
        assert!(res.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn list_space__happy_path() -> anyhow::Result<()> {
        let command_args: SpaceListCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(ListSpacesPayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Ok(Some(())));

        let res = list(command_args, &mut api_client).await?;
        // assert_eq!((), res); //TODO

        Ok(())
    }

    #[tokio::test]
    async fn list_space__err_if_api_client_fails() -> anyhow::Result<()> {
        let command_args: SpaceListCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(ListSpacesPayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Err(ockam::OckamError::BareError.into()));

        let res = list(command_args, &mut api_client).await;
        assert!(res.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn delete_space__happy_path() -> anyhow::Result<()> {
        let command_args: SpaceCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(DeleteSpacePayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Ok(Some(())));

        let res = delete(command_args, &mut api_client).await?;
        // assert_eq!((), res); //TODO

        Ok(())
    }

    #[tokio::test]
    async fn delete_space__err_if_api_client_fails() -> anyhow::Result<()> {
        let command_args: SpaceCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(DeleteSpacePayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Err(ockam::OckamError::BareError.into()));

        let res = delete(command_args, &mut api_client).await;
        assert!(res.is_err());

        Ok(())
    }
}
