use miette::IntoDiagnostic;
use tauri::{AppHandle, Manager, State, Wry};
use tracing::log::{error, info};

use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::CliState;
use ockam_command::enroll::{enroll, Auth0Service};
use ockam_command::CommandGlobalOpts;
use ockam_core::AsyncTryClone;

use crate::app;
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

    let context = app_state.context().async_try_clone().await.unwrap();
    enroll_with_token(&context, app_state.options())
        .await
        .unwrap_or_else(|e| error!("{:?}", e));
    app.trigger_global(app::events::SYSTEM_TRAY_ON_UPDATE, None);
    Ok(())
}

async fn enroll_with_token(ctx: &Context, options: CommandGlobalOpts) -> Result<()> {
    // get an Auth0 token
    let token = Auth0Service::default().get_token_with_pkce().await?;
    // enroll the current user using that token on the controller
    enroll(ctx, &options, token).await?;
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
