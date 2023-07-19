use miette::{miette, IntoDiagnostic};
use tauri::{AppHandle, Manager, State, Wry};
use tracing::log::{error, info};

use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::{CliState, SpaceConfig};
use ockam_api::cloud::enroll::auth0::AuthenticateAuth0Token;
use ockam_api::cloud::space::CreateSpace;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_command::enroll::{retrieve_user_project, Auth0Service};
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
        .unwrap_or_else(|e| error!("{:?}", e));

    app.trigger_global(crate::app::events::SYSTEM_TRAY_ON_UPDATE, None);

    Ok(())
}

async fn enroll_with_token(app_state: &AppState) -> Result<()> {
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

    retrieve_space(app_state).await?;

    retrieve_user_project(&app_state.context(), &app_state.options(), "default")
        .await
        .map(|_| ())
        .into_diagnostic()?;
    Ok(())
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

async fn retrieve_space(app_state: &AppState) -> Result<()> {
    let spaces = app_state
        .node_manager()
        .list_spaces(&app_state.context(), &CloudOpts::route())
        .await
        .map_err(|e| miette!(e))?;
    let space = match spaces.first() {
        Some(space) => space.clone(),
        None => app_state
            .node_manager()
            .create_space(
                &app_state.context(),
                CreateSpace::new("default".to_string(), vec![]),
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

    Ok(())
}
