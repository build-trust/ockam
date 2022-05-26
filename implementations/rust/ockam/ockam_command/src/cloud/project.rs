use clap::Parser;

use ockam::TCP;
use ockam::{Context, TcpTransport};
use ockam_api::cloud::project::CreateProject;
use ockam_core::route;

use crate::old::storage;

pub async fn run(args: ProjectCommand, ctx: &mut Context) -> anyhow::Result<()> {
    storage::ensure_identity_exists(true)?;
    let _ockam_dir = storage::get_ockam_dir()?;

    TcpTransport::create(ctx).await?;

    let addr = "cloud.ockam.io:62526"; //TODO
    let route = route![(TCP, addr), "projects"]; //TODO
    let mut api_client = ockam_api::cloud::Client::new(route, ctx).await?;

    match args.command {
        ProjectsSubCommand::Create {
            space_id,
            project_name,
            services,
        } => {
            let res = api_client
                .create_project(&space_id, CreateProject::new(project_name, &services))
                .await?;
            println!("{res:?}")
        }
        ProjectsSubCommand::List { space_id } => {
            let res = api_client.list_projects(&space_id).await?;
            println!("{res:?}")
        }
        ProjectsSubCommand::Show {
            space_id,
            project_id,
        } => {
            let res = api_client.get_project(&space_id, &project_id).await?;
            println!("{res:?}")
        }
        ProjectsSubCommand::Delete {
            space_id,
            project_id,
        } => {
            let res = api_client.delete_project(&space_id, &project_id).await?;
            println!("{res:?}")
        }
    };
    ctx.stop().await?;
    Ok(())
}

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
    Create {
        /// Id of the space the project belongs to.
        space_id: String,
        /// Name of the project.
        project_name: String,
        /// Services enabled for this project.
        services: Vec<String>,
    },
    /// List all projects.
    #[clap(display_order = 1001)]
    List {
        /// Id of the space.
        space_id: String,
    },
    /// Shows a single project.
    #[clap(display_order = 1002)]
    Show {
        /// Id of the space.
        space_id: String,
        /// Id of the project.
        project_id: String,
    },
    /// Delete a project.
    #[clap(display_order = 1003)]
    Delete {
        /// Id of the space.
        space_id: String,
        /// Id of the project.
        project_id: String,
    },
}
