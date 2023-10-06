use indicatif::ProgressBar;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use std::iter::Take;
use std::time::Duration;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;
use tracing::debug;

use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::{Project, Projects};
use ockam_api::cloud::{Controller, ORCHESTRATOR_AWAIT_TIMEOUT_MS};
use ockam_api::config::lookup::LookupMeta;
use ockam_api::error::ApiError;
use ockam_api::nodes::service::relay::SecureChannelsCreation;
use ockam_api::nodes::InMemoryNode;

use ockam_api::route_to_multiaddr;
use ockam_core::compat::str::FromStr;
use ockam_core::route;
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::Context;

use crate::{CommandGlobalOpts, Result};

pub fn clean_projects_multiaddr(
    input: MultiAddr,
    projects_secure_channels: Vec<MultiAddr>,
) -> Result<MultiAddr> {
    let mut new_ma = MultiAddr::default();
    let mut sc_iter = projects_secure_channels.iter().peekable();
    for p in input.iter().peekable() {
        match p.code() {
            ockam_multiaddr::proto::Project::CODE => {
                let alias = p
                    .cast::<ockam_multiaddr::proto::Project>()
                    .ok_or_else(|| miette!("Invalid project value"))?;
                let sc = sc_iter
                    .next()
                    .ok_or_else(|| miette!("Missing secure channel for project {}", &*alias))?;
                for v in sc.iter().peekable() {
                    new_ma.push_back_value(&v)?;
                }
            }
            _ => new_ma.push_back_value(&p)?,
        }
    }
    debug!(%input, %new_ma, "Projects names replaced with secure channels");
    Ok(new_ma)
}

pub async fn get_projects_secure_channels_from_config_lookup(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &impl SecureChannelsCreation,
    meta: &LookupMeta,
    identity_name: Option<String>,
    timeout: Option<Duration>,
) -> Result<Vec<MultiAddr>> {
    let mut sc = Vec::with_capacity(meta.project.len());

    // Create a secure channel for each project.
    for name in meta.project.iter() {
        // Get the project node's access route + identity id from the config
        let (project_access_route, project_identity_id) = {
            // This shouldn't fail, as we did a refresh above if we found any missing project.
            let p = opts
                .state
                .projects
                .get(name)
                .context(format!("Failed to get project {name} from config lookup"))?
                .config()
                .clone();
            let id = p
                .identity
                .ok_or(miette!("Project should have identity set"))?;
            let node_route = MultiAddr::from_str(&p.access_route)
                .into_diagnostic()
                .wrap_err("Invalid project node route")?;
            (node_route, id)
        };

        debug!("creating a secure channel to {project_access_route}");
        let secure_channel = node
            .create_secure_channel(
                ctx,
                &project_access_route,
                project_identity_id,
                identity_name.clone(),
                None,
                timeout,
            )
            .await?;
        let address = route_to_multiaddr(&route![secure_channel.to_string()])
            .ok_or_else(|| ApiError::core(format!("Invalid route: {}", secure_channel)))?;
        debug!("secure channel created at {address}");
        sc.push(address);
    }

    // There should be the same number of project occurrences in the
    // input MultiAddr than there are in the secure channels vector.
    assert_eq!(meta.project.len(), sc.len());
    Ok(sc)
}

pub async fn check_project_readiness(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    project: Project,
) -> Result<Project> {
    // Total of 10 Mins sleep strategy with 5 second intervals between each retry
    let retry_strategy =
        FixedInterval::from_millis(5000).take(ORCHESTRATOR_AWAIT_TIMEOUT_MS / 5000);

    // Persist project config prior to checking readiness which might take a while
    opts.state
        .projects
        .overwrite(&project.name, project.clone())?;

    let spinner_option = opts.terminal.progress_spinner();
    let project = check_project_ready(
        ctx,
        &node.create_controller().await?,
        project,
        retry_strategy.clone(),
        spinner_option.clone(),
    )
    .await?;
    let project = check_project_node_accessible(
        ctx,
        node,
        project,
        retry_strategy.clone(),
        spinner_option.clone(),
    )
    .await?;
    let project =
        check_authority_node_accessible(ctx, node, project, retry_strategy, spinner_option.clone())
            .await?;

    if let Some(spinner) = spinner_option.as_ref() {
        spinner.finish_and_clear();
    }

    // Persist project config with all its fields
    opts.state
        .projects
        .overwrite(&project.name, project.clone())?;
    Ok(project)
}

async fn check_project_ready(
    ctx: &Context,
    controller: &Controller,
    project: Project,
    retry_strategy: Take<FixedInterval>,
    spinner_option: Option<ProgressBar>,
) -> Result<Project> {
    if let Some(spinner) = spinner_option.as_ref() {
        spinner.set_message("Waiting for project to be ready...");
    }

    // Check if Project and Project Authority info is available
    if project.is_ready() {
        return Ok(project);
    };

    let project_id = project.id.clone();
    let project: Project = Retry::spawn(retry_strategy.clone(), || async {
        // Handle the project show request result
        // so we can provide better errors in the case orchestrator does not respond timely
        let project = controller.get_project(ctx, project_id.clone()).await?;
        let result: miette::Result<Project> = if project.is_ready() {
            Ok(project)
        } else {
            Err(miette!("Project creation timed out. Please try again."))
        };
        result
    })
    .await?;
    Ok(project)
}

async fn check_project_node_accessible(
    ctx: &Context,
    node: &InMemoryNode,
    project: Project,
    retry_strategy: Take<FixedInterval>,
    spinner_option: Option<ProgressBar>,
) -> Result<Project> {
    let project_route = project.access_route()?;
    let project_identity = project
        .identity
        .as_ref()
        .ok_or(miette!("Project identity is not set."))?;
    let project_node = node
        .create_project_client(project_identity, &project_route, None)
        .await?;

    if let Some(spinner) = spinner_option.as_ref() {
        spinner.set_message("Establishing connection to the project...");
    }

    Retry::spawn(retry_strategy.clone(), || async {
        // Handle the reachable result, so we can provide better errors in the case a project isn't
        if let Ok(reachable) = project.is_reachable().await {
            if reachable {
                return Ok(());
            }
        }
        Err(miette!(
            "Timed out while trying to establish a connection to the project. Please try again."
        ))
    })
    .await?;

    if let Some(spinner) = spinner_option.as_ref() {
        spinner.set_message("Establishing secure channel to project...");
    }

    Retry::spawn(retry_strategy.clone(), || async {
        if project_node.check_secure_channel(ctx).await.is_ok() {
            Ok(())
        } else {
            Err(miette!("Timed out while trying to establish a secure channel to the project. Please try again."))
        }
    })
        .await?;

    Ok(project)
}

async fn check_authority_node_accessible(
    ctx: &Context,
    node: &InMemoryNode,
    project: Project,
    retry_strategy: Take<FixedInterval>,
    spinner_option: Option<ProgressBar>,
) -> Result<Project> {
    let authority = project
        .authority()
        .await?
        .ok_or(miette!("Project does not have an authority defined."))?;

    let authority_node = node
        .create_authority_client(authority.identity_id(), authority.address(), None)
        .await?;

    if let Some(spinner) = spinner_option.as_ref() {
        spinner.set_message("Establishing secure channel to project authority...");
    }
    Retry::spawn(retry_strategy.clone(), || async {
        if authority_node.check_secure_channel(ctx).await.is_ok() {
                Ok(())
            } else {
                Err(miette!("Timed out while trying to establish a secure channel to the project authority. Please try again."))
            }
        })
            .await?;
    Ok(project)
}

pub async fn refresh_projects(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    controller: &Controller,
) -> miette::Result<()> {
    let projects = controller.list_projects(ctx).await?;
    for project in projects {
        opts.state
            .projects
            .overwrite(&project.name, project.clone())?;
    }
    Ok(())
}
