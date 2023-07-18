use tauri::{AppHandle, Manager, State, Wry};
use tracing::log::{error, info};

use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_command::enroll::{enroll, Auth0Service};
use ockam_command::identity::{create_default_identity, CreateCommand};
use ockam_command::CommandGlobalOpts;
use ockam_core::AsyncTryClone;

use crate::app;
use crate::app::AppState;
use crate::Result;

/// Enroll a user.
/// This function:
///  - creates a default node, with a default identity, if it doesn't exist
///  - connects to the Auth0 service to authenticate the user of the Ockam application to retrieve a token
///  - connects to the Orchestrator with the retrieved token to create a project
pub async fn enroll_user(app: &AppHandle<Wry>) -> Result<()> {
    info!("starting to enroll the user");
    let app_state: State<AppState> = app.state::<AppState>();
    if app_state.state.identities.default().is_err() {
        let create_command = CreateCommand::new("default".to_string(), None);
        create_command.create_identity(app_state.options()).await?;
    }

    info!("got the default identity");
    let context = app_state.context.async_try_clone().await.unwrap();
    match enroll_with_token(&context, app_state.options()).await {
        Ok(_) => (),
        Err(e) => error!("{:?}", e),
    };
    app.trigger_global(app::events::SYSTEM_TRAY_ON_UPDATE, None);
    Ok(())
}

async fn enroll_with_token(ctx: &Context, options: CommandGlobalOpts) -> Result<()> {
    // get an Auth0 token
    let token = Auth0Service::default().get_token_with_pkce().await?;
    // enroll the current user using that token on the controller
    enroll(&ctx, &options, token).await?;
    Ok(())
}
