use anyhow::anyhow;
use clap::{Args, Parser};
#[cfg(test)]
use fake::{Dummy, Fake, Faker};

use ockam::Context;

use crate::api;
use crate::api::{
    CloudApi, CreateProjectPayload, DeleteProjectRequestPayload, ListProjectsRequestPayload,
    RequestMethod, ShowProjectRequestPayload,
};

#[derive(Clone, Debug, Parser)]
pub struct ProjectCommand {
    #[clap(subcommand)]
    pub command: ProjectsSubCommand,
    #[clap(long, short, parse(from_occurrences))]
    pub verbose: u8,
}

#[derive(Clone, Debug, Parser)]
pub enum ProjectsSubCommand {
    /// Creates a new project.
    #[clap(display_order = 1000)]
    Create(ProjectCommandArgs),
    /// List all projects.
    #[clap(display_order = 1001)]
    List(ProjectListOpts),
    /// Shows a single project.
    #[clap(display_order = 1002)]
    Show(ProjectCommandArgs),
    /// Delete a project.
    #[clap(display_order = 1003)]
    Delete(ProjectCommandArgs),
}

#[derive(Clone, Debug, Args)]
#[cfg_attr(test, derive(Dummy))]
pub struct ProjectCommandArgs {
    /// Name of the project.
    pub project_name: String,
}

#[derive(Clone, Debug, Args)]
#[cfg_attr(test, derive(Dummy))]
pub struct ProjectListOpts;

pub async fn run(args: ProjectCommand, mut ctx: Context) -> anyhow::Result<()> {
    let mut api_client = api::NodeCloudApi::from(&mut ctx);
    let res = match args.command {
        ProjectsSubCommand::Create(arg) => create(arg, &mut api_client).await,
        ProjectsSubCommand::List(arg) => list(arg, &mut api_client).await,
        ProjectsSubCommand::Show(arg) => show(arg, &mut api_client).await,
        ProjectsSubCommand::Delete(arg) => delete(arg, &mut api_client).await,
    };
    api_client.ctx.stop().await?;
    res
}

pub async fn create<T>(args: ProjectCommandArgs, api_client: &mut T) -> anyhow::Result<()>
where
    T: CloudApi<api::project::create::RequestParams, api::project::create::RequestBody>,
{
    match api_client
        .send::<api::project::create::ResponseBody>(
            RequestMethod::Put,
            api::project::create::RequestParams::from(args),
            api::project::create::RequestBody::from(args),
        )
        .await
    {
        Ok(r) => {
            tracing::info!("{:?}", r);
            println!("Project created successfully");
            Ok(())
        }
        Err(err) => {
            tracing::error!("{:?}", err);
            Err(anyhow!("Failed to create project"))
        }
    }
}

pub async fn list<T>(_args: ProjectListOpts, api_client: &mut T) -> anyhow::Result<()>
where
    T: CloudApi<ListProjectsRequestPayload>,
{
    //TODO: return type should be a struct representing the list of items.
    match api_client.send::<()>(ListProjectsRequestPayload).await {
        Ok(_res) => {
            // TODO
            // if let Some(res) = res {
            // println!("{res}");
            // };
            Ok(())
        }
        Err(err) => {
            tracing::error!("{:?}", err);
            Err(anyhow!("Failed to retrieve projects"))
        }
    }
}

pub async fn show<T>(args: ProjectCommandArgs, api_client: &mut T) -> anyhow::Result<()>
where
    T: CloudApi<ShowProjectRequestPayload>,
{
    //TODO: return type should be a struct representing the retrieved item.
    match api_client
        .send::<()>(ShowProjectRequestPayload::from(args))
        .await
    {
        Ok(_res) => {
            // TODO
            // if let Some(res) = res {
            // println!("{res}");
            // };
            Ok(())
        }
        Err(err) => {
            tracing::error!("{:?}", err);
            Err(anyhow!("Failed to retrieve project"))
        }
    }
}

pub async fn delete<T>(args: ProjectCommandArgs, api_client: &mut T) -> anyhow::Result<()>
where
    T: CloudApi<DeleteProjectRequestPayload>,
{
    match api_client
        .send::<()>(DeleteProjectRequestPayload::from(args))
        .await
    {
        Ok(_) => {
            println!("Project deleted successfully");
            Ok(())
        }
        Err(err) => {
            tracing::error!("{:?}", err);
            Err(anyhow!("Failed to delete project"))
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

            let payload: CreateProjectPayload = Faker.fake();
            send_payload::<CreateProjectPayload>(&mut node_api, &worker_name, payload).await?;

            let payload: ShowProjectRequestPayload = Faker.fake();
            send_payload::<ShowProjectRequestPayload>(&mut node_api, &worker_name, payload).await?;

            let payload: ListProjectsRequestPayload = Faker.fake();
            send_payload::<ListProjectsRequestPayload>(&mut node_api, &worker_name, payload)
                .await?;

            let payload: DeleteProjectRequestPayload = Faker.fake();
            send_payload::<DeleteProjectRequestPayload>(&mut node_api, &worker_name, payload)
                .await?;

            ctx.stop().await?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn create_project__happy_path() -> anyhow::Result<()> {
        let command_args: ProjectCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(CreateProjectPayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Ok(Some(())));

        let res = create(command_args, &mut api_client).await?;
        // assert_eq!((), res); //TODO

        Ok(())
    }

    #[tokio::test]
    async fn create_project__err_if_api_client_fails() -> anyhow::Result<()> {
        let command_args: ProjectCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(CreateProjectPayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Err(ockam::OckamError::BareError.into()));

        let res = create(command_args, &mut api_client).await;
        assert!(res.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn show_project__happy_path() -> anyhow::Result<()> {
        let command_args: ProjectCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(ShowProjectRequestPayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Ok(Some(())));

        let res = show(command_args, &mut api_client).await?;
        // assert_eq!((), res); //TODO

        Ok(())
    }

    #[tokio::test]
    async fn show_project__err_if_api_client_fails() -> anyhow::Result<()> {
        let command_args: ProjectCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(ShowProjectRequestPayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Err(ockam::OckamError::BareError.into()));

        let res = show(command_args, &mut api_client).await;
        assert!(res.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn list_project__happy_path() -> anyhow::Result<()> {
        let command_args: ProjectListOpts = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(ListProjectsRequestPayload))
            .times(1)
            .returning(|_| Ok(Some(())));

        let res = list(command_args, &mut api_client).await?;
        // assert_eq!((), res); //TODO

        Ok(())
    }

    #[tokio::test]
    async fn list_project__err_if_api_client_fails() -> anyhow::Result<()> {
        let command_args: ProjectListOpts = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(ListProjectsRequestPayload))
            .times(1)
            .returning(|_| Err(ockam::OckamError::BareError.into()));

        let res = list(command_args, &mut api_client).await;
        assert!(res.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn delete_project__happy_path() -> anyhow::Result<()> {
        let command_args: ProjectCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(DeleteProjectRequestPayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Ok(Some(())));

        let res = delete(command_args, &mut api_client).await?;
        // assert_eq!((), res); //TODO

        Ok(())
    }

    #[tokio::test]
    async fn delete_project__err_if_api_client_fails() -> anyhow::Result<()> {
        let command_args: ProjectCommandArgs = Faker.fake();
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<()>()
            .with(eq(DeleteProjectRequestPayload::from(command_args.clone())))
            .times(1)
            .returning(|_| Err(ockam::OckamError::BareError.into()));

        let res = delete(command_args, &mut api_client).await;
        assert!(res.is_err());

        Ok(())
    }
}
