use miette::{miette, IntoDiagnostic};

use tauri::{AppHandle, Manager, State, Wry};
use tracing::log::{error, info};

use ockam::identity::IdentityIdentifier;
use ockam_api::cli_state;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::SpaceConfig;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::space::{CreateSpace, Space};
use ockam_command::enroll::{update_enrolled_identity, OidcService};
use ockam_command::util::api::CloudOpts;

use crate::app::{AppState, NODE_NAME, PROJECT_NAME};
use crate::Result;

/// Enroll a user.
///
/// This function:
///  - creates a default node, with a default identity if it doesn't exist
///  - connects to the Auth0 service to authenticate the user of the Ockam application to retrieve a token
///  - connects to the Orchestrator with the retrieved token to create a project
pub async fn enroll_user(app: &AppHandle<Wry>) -> Result<()> {
    let app_state: State<AppState> = app.state::<AppState>();
    enroll_with_token(&app_state)
        .await
        .map(|i| info!("Enrolled a new user with identifier {}", i))
        .unwrap_or_else(|e| error!("{:?}", e));
    create_relay(&app_state).await?;
    app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);
    Ok(())
}

async fn enroll_with_token(app_state: &AppState) -> Result<IdentityIdentifier> {
    // get an Auth0 token
    let auth0_service = OidcService::default();
    let token = auth0_service.get_token_with_pkce().await?;

    // retrieve the user information
    let user_info = auth0_service.get_user_info(token.clone()).await?;
    info!("the user info is {user_info:?}");
    app_state.set_user_info(user_info).await?;

    // enroll the current user using that token on the controller
    let node_manager = app_state.node_manager.get().write().await;
    node_manager
        .enroll_auth0(&app_state.context(), &CloudOpts::route(), token)
        .await
        .into_diagnostic()?;

    let space = retrieve_space(app_state).await?;
    let _ = retrieve_project(app_state, &space).await?;
    let identifier = update_enrolled_identity(&app_state.options().await, NODE_NAME)
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

    let identity_state = state
        .create_identity_state(&identity.identifier(), None)
        .await?;
    info!("created a new identity state");
    state.identities.set_default(identity_state.name())?;
    Ok(())
}

async fn retrieve_space(app_state: &AppState) -> Result<Space> {
    info!("retrieving the user space");
    let node_manager = app_state.node_manager.get().read().await;

    // list the spaces that the user can access
    // and sort them by name to make sure to get the same space every time
    // if several spaces are available
    let spaces = {
        let mut spaces = node_manager
            .list_spaces(&app_state.context(), &CloudOpts::route())
            .await
            .map_err(|e| miette!(e))?;
        spaces.sort_by(|s1, s2| s1.name.cmp(&s2.name));
        spaces
    };

    // take the first one that is available
    // otherwise create a space with a random name
    let space = match spaces.first() {
        Some(space) => space.clone(),
        None => {
            let space_name = cli_state::random_name();
            node_manager
                .create_space(
                    &app_state.context(),
                    CreateSpace::new(space_name, vec![]),
                    &CloudOpts::route(),
                    None,
                )
                .await
                .map_err(|e| miette!(e))?
        }
    };
    app_state
        .state()
        .await
        .spaces
        .overwrite(&space.name, SpaceConfig::from(&space))?;

    Ok(space)
}

async fn retrieve_project(app_state: &AppState, space: &Space) -> Result<Project> {
    info!("retrieving the user project");
    let node_manager = app_state.node_manager.get().read().await;
    let projects = {
        node_manager
            .list_projects(&app_state.context(), &CloudOpts::route())
            .await
            .map_err(|e| miette!(e))?
    };
    let project = match projects.iter().find(|p| p.name == *PROJECT_NAME) {
        Some(project) => project.clone(),
        None => node_manager
            .create_project(
                &app_state.context(),
                &CloudOpts::route(),
                space.id.as_str(),
                PROJECT_NAME,
                vec![],
            )
            .await
            .map_err(|e| miette!(e))?,
    };
    app_state
        .state()
        .await
        .projects
        .overwrite(&project.name, project.clone())?;

    Ok(project)
}

async fn create_relay(_app_state: &AppState) -> Result<()> {
    // TODO: the relay creation fails because the NodeManagerWorker is initialized before the
    //       enrollment, hence, it's not initialized with the project/trust-context info.
    //       We need to figure out how to update the NodeManagerWorker after the enrollment is done.
    // let project_route = app_state
    //     .state()
    //     .projects
    //     .default()?
    //     .config()
    //     .access_route
    //     .clone();
    // let project_address = MultiAddr::from_str(&project_route).into_diagnostic()?;
    // let req = CreateForwarder::at_project(project_address.clone(), None);
    // app_state
    //     .node_manager
    //     .create_forwarder(&app_state.context(), req)
    //     .await
    //     .into_diagnostic()?;
    Ok(())
}
