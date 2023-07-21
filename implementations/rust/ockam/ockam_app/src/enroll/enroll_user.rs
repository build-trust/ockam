use miette::{miette, IntoDiagnostic};
use ockam::identity::IdentityIdentifier;
use tauri::{AppHandle, Manager, State, Wry};
use tracing::log::{error, info};

use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::{CliState, SpaceConfig};
use ockam_api::cloud::enroll::auth0::AuthenticateAuth0Token;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::space::{CreateSpace, Space};
use ockam_api::cloud::CloudRequestWrapper;
use ockam_command::enroll::{update_enrolled_identity, Auth0Service};
use ockam_command::util::api::CloudOpts;

use crate::app::AppState;
use crate::Result;

/// Enroll a user.
///
/// This function:
///  - creates a default node, with a default identity if it doesn't exist
///  - connects to the Auth0 service to authenticate the user of the Ockam application to retrieve a token
///  - connects to the Orchestrator with the retrieved token to create a project
pub async fn enroll_user(app: &AppHandle<Wry>) -> Result<()> {
    let app_state: State<AppState> = app.state::<AppState>();
    if app_state.state().identities.default().is_err() {
        info!("creating a default identity");
        create_default_identity(app_state.state())
            .await
            .map_err(|e| {
                error!("{:?}", e);
                e
            })?;
    };

    enroll_with_token(&app_state)
        .await
        .map(|i| info!("Enrolled a new user with identifier {}", i))
        .unwrap_or_else(|e| error!("{:?}", e));

    app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);

    Ok(())
}

async fn enroll_with_token(app_state: &AppState) -> Result<IdentityIdentifier> {
    // get an Auth0 token
    let token = Auth0Service::default().get_token_with_pkce().await?;
    // enroll the current user using that token on the controller
    let request = CloudRequestWrapper::new(
        AuthenticateAuth0Token::new(token),
        &CloudOpts::route(),
        None,
    );
    app_state
        .node_manager()
        .enroll_auth0(&app_state.context(), request)
        .await
        .into_diagnostic()?;

    let space = retrieve_space(app_state).await?;
    let _ = retrieve_project(app_state, &space).await?;
    let identifier = update_enrolled_identity(&app_state.options(), "default")
        .await
        .into_diagnostic()?;
    Ok(identifier)
}

async fn create_default_identity(state: CliState) -> Result<()> {
    let vault_state = match state.vaults.default() {
        Err(_) => {
            info!("creating a default vault");
            state.create_vault_state(None).await?
        }
        Ok(vault_state) => vault_state,
    };
    info!("retrieved the vault state");

    let identity = state
        .get_identities(vault_state.get().await?)
        .await?
        .identities_creation()
        .create_identity()
        .await
        .into_diagnostic()?;
    info!("created a new identity {}", identity.identifier());

    let _ = state
        .create_identity_state(&identity.identifier(), None)
        .await?;
    info!("created a new identity state");
    Ok(())
}

async fn retrieve_space(app_state: &AppState) -> Result<Space> {
    let default_space_name = "default".to_string();
    let spaces = app_state
        .node_manager()
        .list_spaces(&app_state.context(), &CloudOpts::route())
        .await
        .map_err(|e| miette!(e))?;
    let space = match spaces.iter().find(|s| s.name == default_space_name) {
        Some(space) => space.clone(),
        None => app_state
            .node_manager()
            .create_space(
                &app_state.context(),
                CreateSpace::new(default_space_name, vec![]),
                &CloudOpts::route(),
                None,
            )
            .await
            .map_err(|e| miette!(e))?,
    };
    app_state
        .state()
        .spaces
        .overwrite(&space.name, SpaceConfig::from(&space))?;

    Ok(space)
}

async fn retrieve_project(app_state: &AppState, space: &Space) -> Result<Project> {
    let default_project_name = "default";
    let projects = app_state
        .node_manager()
        .list_projects(&app_state.context(), &CloudOpts::route())
        .await
        .map_err(|e| miette!(e))?;
    let project = match projects.iter().find(|p| p.name == default_project_name) {
        Some(project) => project.clone(),
        None => app_state
            .node_manager()
            .create_project(
                &app_state.context(),
                &CloudOpts::route(),
                space.id.as_str(),
                default_project_name,
                vec![],
            )
            .await
            .map_err(|e| miette!(e))?,
    };
    app_state
        .state()
        .projects
        .overwrite(&project.name, project.clone())?;

    Ok(project)
}
