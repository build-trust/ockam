use crate::app::State;
use crate::AppHandle;
use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_command::enroll::{enroll, Auth0Service};
use ockam_command::identity::create_default_identity;
use ockam_command::util::embedded_node;
use ockam_command::{CommandGlobalOpts, GlobalArgs};
use tauri::Manager;

/// Enroll a user.
/// This function:
///  - creates a default node, with a default identity, if it doesn't exist
///  - connects to the Auth0 service to authenticate the user of the Ockam application to retrieve a token
///  - connects to the Orchestrator with the retrieved token to create a project
#[tauri::command]
pub fn enroll_user() {
    let args = GlobalArgs::default().set_quiet();
    let options = CommandGlobalOpts::new(args);
    if options.state.identities.default().is_err() {
        create_default_identity(&options);
    }
    if let Err(e) = embedded_node(rpc, options) {
        println!("Error while enrolling user: {e:?}");
    }
    // Update tray menu
    let state = app_handle.state::<State>();
    {
        let mut tray_menu = state.tray_menu.write().unwrap();
        tray_menu
            .options
            .reset
            .set_enabled(&app_handle.tray_handle(), true);
        tray_menu
            .enroll
            .enroll
            .set_enabled(&app_handle.tray_handle(), false);
    }
}

async fn rpc(ctx: Context, options: CommandGlobalOpts) -> miette::Result<()> {
    // get an Auth0 token
    let token = Auth0Service::default().get_token_with_pkce().await?;
    // enroll the current user using that token on the controller
    enroll(&ctx, &options, token).await?;
    Ok(())
}
