use anyhow::anyhow;
use clap::{Args, Parser};
#[cfg(test)]
use fake::{Dummy, Fake, Faker};

use ockam::Context;

use crate::api::{project, CloudApi, NodeCloudApi, RequestMethod};

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
    /// Name of the space the project belongs to.
    pub space_name: String,
    /// Name of the project.
    pub project_name: String,
    /// Services enabled for this project.
    pub services: Vec<String>,
}

#[derive(Clone, Debug, Args)]
#[cfg_attr(test, derive(Dummy))]
pub struct ProjectListOpts;

pub async fn run(args: ProjectCommand, mut ctx: Context) -> anyhow::Result<()> {
    let mut api_client = NodeCloudApi::from(&mut ctx);
    let res = match args.command {
        ProjectsSubCommand::Create(arg) => create(arg, &mut api_client).await,
        // ProjectsSubCommand::List(arg) => list(arg, &mut api_client).await,
        // ProjectsSubCommand::Show(arg) => show(arg, &mut api_client).await,
        // ProjectsSubCommand::Delete(arg) => delete(arg, &mut api_client).await,
        _ => Ok(()),
    };
    api_client.ctx.stop().await?;
    res
}

pub async fn create<Api>(args: ProjectCommandArgs, api_client: &mut Api) -> anyhow::Result<()>
where
    Api: CloudApi,
{
    let ProjectCommandArgs {
        space_name,
        project_name,
        services,
    } = args;
    let params = project::create::RequestParams { space_name };
    let body = project::create::RequestBody {
        project_name,
        services,
    };
    match api_client
        .send::<_, _, project::create::ResponseBody>(RequestMethod::Put, params, body)
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

// pub async fn list<T>(_args: ProjectListOpts, api_client: &mut T) -> anyhow::Result<()>
// where
//     T: CloudApi<ListProjectsRequestPayload>,
// {
//     //TODO: return type should be a struct representing the list of items.
//     match api_client.send::<()>(ListProjectsRequestPayload).await {
//         Ok(_res) => {
//             // TODO
//             // if let Some(res) = res {
//             // println!("{res}");
//             // };
//             Ok(())
//         }
//         Err(err) => {
//             tracing::error!("{:?}", err);
//             Err(anyhow!("Failed to retrieve projects"))
//         }
//     }
// }
//
// pub async fn show<T>(args: ProjectCommandArgs, api_client: &mut T) -> anyhow::Result<()>
// where
//     T: CloudApi<ShowProjectRequestPayload>,
// {
//     //TODO: return type should be a struct representing the retrieved item.
//     match api_client
//         .send::<()>(ShowProjectRequestPayload::from(args))
//         .await
//     {
//         Ok(_res) => {
//             // TODO
//             // if let Some(res) = res {
//             // println!("{res}");
//             // };
//             Ok(())
//         }
//         Err(err) => {
//             tracing::error!("{:?}", err);
//             Err(anyhow!("Failed to retrieve project"))
//         }
//     }
// }
//
// pub async fn delete<T>(args: ProjectCommandArgs, api_client: &mut T) -> anyhow::Result<()>
// where
//     T: CloudApi<DeleteProjectRequestPayload>,
// {
//     match api_client
//         .send::<()>(DeleteProjectRequestPayload::from(args))
//         .await
//     {
//         Ok(_) => {
//             println!("Project deleted successfully");
//             Ok(())
//         }
//         Err(err) => {
//             tracing::error!("{:?}", err);
//             Err(anyhow!("Failed to delete project"))
//         }
//     }
// }

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use mockall::predicate::*;

    use crate::api::MockCloudApi;

    use super::*;

    mod node_api {
        use crate::api::tests::node_api::*;

        use super::*;

        #[ockam::test(crate = "ockam")]
        async fn can_send_payloads(ctx: &mut ockam::Context) -> ockam::Result<()> {
            let (mut node_api, worker_name) = setup_node_api(ctx).await?;

            let payload: project::create::RequestBody = Faker.fake();
            send_payload::<project::create::RequestBody>(&mut node_api, &worker_name, payload)
                .await?;

            // let payload: ShowProjectRequestPayload = Faker.fake();
            // send_payload::<ShowProjectRequestPayload>(&mut node_api, &worker_name, payload).await?;
            //
            // let payload: ListProjectsRequestPayload = Faker.fake();
            // send_payload::<ListProjectsRequestPayload>(&mut node_api, &worker_name, payload)
            //     .await?;
            //
            // let payload: DeleteProjectRequestPayload = Faker.fake();
            // send_payload::<DeleteProjectRequestPayload>(&mut node_api, &worker_name, payload)
            //     .await?;

            ctx.stop().await?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn create_project__happy_path() -> anyhow::Result<()> {
        let command_args: ProjectCommandArgs = Faker.fake();
        let ProjectCommandArgs {
            space_name,
            project_name,
            services,
        } = command_args.clone();
        let params = project::create::RequestParams { space_name };
        let body = project::create::RequestBody {
            project_name,
            services,
        };
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<_, _, project::create::ResponseBody>()
            .with(eq(RequestMethod::Put), eq(params), eq(body))
            .times(1)
            .returning(|_, _, _| Ok(None));

        create(command_args, &mut api_client).await?;

        Ok(())
    }

    #[tokio::test]
    async fn create_project__err_if_api_client_fails() -> anyhow::Result<()> {
        let command_args: ProjectCommandArgs = Faker.fake();
        let ProjectCommandArgs {
            space_name,
            project_name,
            services,
        } = command_args.clone();
        let params = project::create::RequestParams { space_name };
        let body = project::create::RequestBody {
            project_name,
            services,
        };
        let mut api_client = MockCloudApi::new();
        api_client
            .expect_send::<_, _, project::create::ResponseBody>()
            .with(eq(RequestMethod::Put), eq(params), eq(body))
            .times(1)
            .returning(|_, _, _| Err(ockam::OckamError::BareError.into()));

        let res = create(command_args, &mut api_client).await;
        assert!(res.is_err());

        Ok(())
    }

    //     #[tokio::test]
    //     async fn show_project__happy_path() -> anyhow::Result<()> {
    //         let command_args: ProjectCommandArgs = Faker.fake();
    //         let mut api_client = MockCloudApi::new();
    //         api_client
    //             .expect_send::<()>()
    //             .with(eq(ShowProjectRequestPayload::from(command_args.clone())))
    //             .times(1)
    //             .returning(|_| Ok(Some(())));
    //
    //         let res = show(command_args, &mut api_client).await?;
    //         // assert_eq!((), res); //TODO
    //
    //         Ok(())
    //     }
    //
    //     #[tokio::test]
    //     async fn show_project__err_if_api_client_fails() -> anyhow::Result<()> {
    //         let command_args: ProjectCommandArgs = Faker.fake();
    //         let mut api_client = MockCloudApi::new();
    //         api_client
    //             .expect_send::<()>()
    //             .with(eq(ShowProjectRequestPayload::from(command_args.clone())))
    //             .times(1)
    //             .returning(|_| Err(ockam::OckamError::BareError.into()));
    //
    //         let res = show(command_args, &mut api_client).await;
    //         assert!(res.is_err());
    //
    //         Ok(())
    //     }
    //
    //     #[tokio::test]
    //     async fn list_project__happy_path() -> anyhow::Result<()> {
    //         let command_args: ProjectListOpts = Faker.fake();
    //         let mut api_client = MockCloudApi::new();
    //         api_client
    //             .expect_send::<()>()
    //             .with(eq(ListProjectsRequestPayload))
    //             .times(1)
    //             .returning(|_| Ok(Some(())));
    //
    //         let res = list(command_args, &mut api_client).await?;
    //         // assert_eq!((), res); //TODO
    //
    //         Ok(())
    //     }
    //
    //     #[tokio::test]
    //     async fn list_project__err_if_api_client_fails() -> anyhow::Result<()> {
    //         let command_args: ProjectListOpts = Faker.fake();
    //         let mut api_client = MockCloudApi::new();
    //         api_client
    //             .expect_send::<()>()
    //             .with(eq(ListProjectsRequestPayload))
    //             .times(1)
    //             .returning(|_| Err(ockam::OckamError::BareError.into()));
    //
    //         let res = list(command_args, &mut api_client).await;
    //         assert!(res.is_err());
    //
    //         Ok(())
    //     }
    //
    //     #[tokio::test]
    //     async fn delete_project__happy_path() -> anyhow::Result<()> {
    //         let command_args: ProjectCommandArgs = Faker.fake();
    //         let mut api_client = MockCloudApi::new();
    //         api_client
    //             .expect_send::<()>()
    //             .with(eq(DeleteProjectRequestPayload::from(command_args.clone())))
    //             .times(1)
    //             .returning(|_| Ok(Some(())));
    //
    //         let res = delete(command_args, &mut api_client).await?;
    //         // assert_eq!((), res); //TODO
    //
    //         Ok(())
    //     }
    //
    //     #[tokio::test]
    //     async fn delete_project__err_if_api_client_fails() -> anyhow::Result<()> {
    //         let command_args: ProjectCommandArgs = Faker.fake();
    //         let mut api_client = MockCloudApi::new();
    //         api_client
    //             .expect_send::<()>()
    //             .with(eq(DeleteProjectRequestPayload::from(command_args.clone())))
    //             .times(1)
    //             .returning(|_| Err(ockam::OckamError::BareError.into()));
    //
    //         let res = delete(command_args, &mut api_client).await;
    //         assert!(res.is_err());
    //
    //         Ok(())
    //     }
}
