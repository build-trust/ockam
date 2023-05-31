use ockam_api::{
    cli_state::traits::StateDirTrait,
    cloud::{enroll::auth0::AuthenticateAuth0Token, CloudRequestWrapper},
};
use ockam_command::{
    enroll::{Auth0Provider, Auth0Service},
    identity::create_default_identity,
    node::util::start_embedded_node,
    util::{node_rpc, OckamConfig, RpcBuilder, DEFAULT_CONTROLLER_ADDRESS},
    CommandGlobalOpts, GlobalArgs,
};
use ockam_core::{env::FromString, CowStr};
use ockam_multiaddr::MultiAddr;

#[tauri::command]
pub fn enroll() -> String {
    let config = OckamConfig::load().expect("Failed to load config");
    let options = CommandGlobalOpts::new(GlobalArgs::default(), config);

    if options.state.identities.default().is_err() {
        create_default_identity(&options);
    }

    node_rpc(rpc, options);

    "Enrolled".to_string()
}

async fn rpc(ctx: ockam::Context, options: CommandGlobalOpts) -> ockam_command::error::Result<()> {
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
    let default_addr = MultiAddr::from_string(DEFAULT_CONTROLLER_ADDRESS)?;
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
