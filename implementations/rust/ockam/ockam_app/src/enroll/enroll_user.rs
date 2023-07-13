use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_command::enroll::{enroll, Auth0Service};
use ockam_command::identity::create_default_identity;
use ockam_command::util::embedded_node;
use ockam_command::{CommandGlobalOpts, GlobalArgs};

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
}

async fn rpc(ctx: Context, options: CommandGlobalOpts) -> miette::Result<()> {
    // get an Auth0 token
    let token = Auth0Service::default().get_token_with_pkce().await?;
    // enroll the current user using that token on the controller
    enroll(&ctx, &options, token).await?;
    Ok(())
}
