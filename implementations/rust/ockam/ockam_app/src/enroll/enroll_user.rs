use miette::{miette, IntoDiagnostic, WrapErr};
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_notification::NotificationExt;
use tracing::{debug, error, info};

use ockam_api::cli_state;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::{add_project_info_to_node_state, update_enrolled_identity, SpaceConfig};
use ockam_api::cloud::project::{Project, Projects};
use ockam_api::cloud::space::{Space, Spaces};
use ockam_api::enroll::enrollment::Enrollment;
use ockam_api::enroll::oidc_service::OidcService;

use crate::app::events::{system_tray_on_update, system_tray_on_update_with_enroll_status};
use crate::app::{AppState, NODE_NAME, PROJECT_NAME};
use crate::{shared_service, Result};

/// Enroll a user.
///
/// This function:
///  - creates a default node, with a default identity if it doesn't exist
///  - connects to the OIDC service to authenticate the user of the Ockam application to retrieve a token
///  - connects to the Orchestrator with the retrieved token to create a project
pub async fn enroll_user<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let app_state: State<AppState> = app.state::<AppState>();
    match enroll_with_token(app, &app_state).await {
        Ok(_) => {
            // Reset the node manager to include the project's setup, needed to create the relay.
            // This is necessary because the project data is used in the worker initialization,
            // which can't be rerun manually once the worker is started.
            app_state.reset_node_manager().await?;
            system_tray_on_update(app);

            // Refresh the projects and invitations now that the node has access to the project
            app.trigger_global(crate::projects::events::REFRESH_PROJECTS, None);
            app.trigger_global(crate::invitations::events::REFRESH_INVITATIONS, None);

            // Create the relay
            shared_service::relay::create_relay(
                app_state.context(),
                app_state.state().await,
                app_state.node_manager().await,
            )
            .await;
            system_tray_on_update(app);
        }
        Err(e) => {
            error!(?e, "Failed to enroll user");
        }
    }
    Ok(())
}

async fn enroll_with_token<R: Runtime>(app: &AppHandle<R>, app_state: &AppState) -> Result<()> {
    if app_state.is_enrolled().await.unwrap_or_default() {
        debug!("User is already enrolled");
        return Ok(());
    }
    app.notification()
        .builder()
        .title("Enrolling...")
        .body("Please wait")
        .show()
        .unwrap_or_else(|e| error!(?e, "Failed to create push notification"));
    system_tray_on_update_with_enroll_status(app, "Waiting for token...")?;

    // get an OIDC token
    let oidc_service = OidcService::default();
    let token = oidc_service.get_token_with_pkce().await?;

    // retrieve the user information
    let user_info = oidc_service.get_user_info(&token).await?;
    info!(?user_info, "User info retrieved successfully");
    let cli_state = app_state.state().await;
    cli_state
        .users_info
        .overwrite(&user_info.email, user_info.clone())?;

    // enroll the current user using that token on the controller
    {
        let controller = app_state.controller().await.into_diagnostic()?;
        controller
            .enroll_with_oidc_token(&app_state.context(), token)
            .await?;
    }
    system_tray_on_update_with_enroll_status(app, "Retrieving space...")?;
    let space = retrieve_space(app_state).await?;
    system_tray_on_update_with_enroll_status(app, "Retrieving project...")?;
    retrieve_project(app, app_state, &space).await?;
    let identifier = update_enrolled_identity(&cli_state, NODE_NAME)
        .await
        .into_diagnostic()?;
    info!(%identifier, "User enrolled successfully");
    app.notification()
        .builder()
        .title("Enrolled successfully!")
        .body("You can now use the Ockam app")
        .show()
        .unwrap_or_else(|e| error!(?e, "Failed to create push notification"));
    Ok(())
}

async fn retrieve_space(app_state: &AppState) -> Result<Space> {
    info!("retrieving the user's space");
    let controller = app_state.controller().await.into_diagnostic()?;

    // list the spaces that the user can access
    // and sort them by name to make sure to get the same space every time
    // if several spaces are available
    let spaces = {
        let mut spaces = controller
            .list_spaces(&app_state.context())
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
            controller
                .create_space(&app_state.context(), space_name, vec![])
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

async fn retrieve_project<R: Runtime>(
    app: &AppHandle<R>,
    app_state: &AppState,
    space: &Space,
) -> Result<Project> {
    info!("retrieving the user project");
    let email = app_state
        .user_email()
        .await
        .wrap_err("User info is not valid")?;

    let controller = app_state.controller().await.into_diagnostic()?;
    let projects = controller
        .list_projects(&app_state.context())
        .await
        .map_err(|e| miette!(e))?;
    let admin_project = projects
        .iter()
        .filter(|p| p.has_admin_with_email(&email))
        .find(|p| p.name == *PROJECT_NAME);

    let project = match admin_project {
        Some(project) => project.clone(),
        None => {
            app.notification()
                .builder()
                .title("Creating a new project...")
                .body("This might take a few seconds")
                .show()
                .unwrap_or_else(|e| error!(?e, "Failed to create push notification"));
            let ctx = &app_state.context();
            let project = controller
                .create_project(ctx, space.id.to_string(), PROJECT_NAME.to_string(), vec![])
                .await
                .map_err(|e| miette!(e))?;
            controller.wait_until_project_is_ready(ctx, project).await?
        }
    };
    let cli_state = app_state.state().await;
    cli_state
        .projects
        .overwrite(&project.name, project.clone())?;
    add_project_info_to_node_state(NODE_NAME, &cli_state, None).await?;
    Ok(project)
}
