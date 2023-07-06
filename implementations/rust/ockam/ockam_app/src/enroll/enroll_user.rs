use miette::miette;

use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cloud::{enroll::auth0::AuthenticateAuth0Token, CloudRequestWrapper};
use ockam_command::enroll::{Auth0Provider, Auth0Service};
use ockam_command::identity::create_default_identity;
use ockam_command::node::util::start_embedded_node;
use ockam_command::util::{node_rpc, RpcBuilder, DEFAULT_CONTROLLER_ADDRESS};
use ockam_command::{CommandGlobalOpts, GlobalArgs};
use ockam_core::{env::FromString, CowStr};
use ockam_multiaddr::MultiAddr;

/// Enroll a user.
/// This function:
///  - creates a default node, with a default identity, if it doesn't exist
///  - connects to the Auth0 service to authenticate the user of the Ockam application to retrieve a token
///  - connects to the Orchestrator with the retrieved token to create a project
///
#[tauri::command]
pub fn enroll_user() {
    let options = CommandGlobalOpts::new(GlobalArgs::default());
    if options.state.identities.default().is_err() {
        create_default_identity(&options);
    }
    node_rpc(rpc, options)
}

async fn rpc(ctx: Context, options: CommandGlobalOpts) -> miette::Result<()> {
    let auth0 = Auth0Service::new(Auth0Provider::Auth0);
    let dc = auth0.device_code().await?;
    let uri: &str = &dc.verification_uri_complete;
    println!("     │ Opening {}", uri);
    if open::that(uri).is_err() {
        println!("Couldn't open activation url automatically [url={}]", uri);
    }

    let token = auth0.poll_token(dc, &options).await?;
    let node_name = start_embedded_node(&ctx, &options, None).await?;
    let mut rpc = RpcBuilder::new(&ctx, &options, &node_name).build();
    let default_addr = MultiAddr::from_string(DEFAULT_CONTROLLER_ADDRESS)
        .map_err(|e| miette!("The default controller address is incorrect: {:?}", e))?;
    let token = AuthenticateAuth0Token::new(token);
    let request = ockam_core::api::Request::post("v0/enroll/auth0").body(CloudRequestWrapper::new(
        token,
        &default_addr,
        None::<CowStr>,
    ));
    rpc.request(request).await?;
    let (_res, _dec) = rpc.check_response()?;

    Ok(())
}
