use ockam_api::nodes::authority_node::{start_node, Configuration};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{AsyncTryClone, Error, Result};
use ockam_node::Context;
use std::env;
use tracing::error;

/// This Ockam node supports all the necessary services required to run an Authority service for a
/// given project:
///  - to issue and validate tokens
///  - to issue credentials for a given project member
///  - to add members to a project (this can only be done by identities with the enroller role)
///  - to authenticate an identity via Okta (using a token) and retrieve its attributes
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // read the configuration file
    let configuration = Configuration::read(&get_configuration_path()?)?;

    match start_node(&ctx, configuration).await {
        Err(e) => {
            error!("Error {e:?}");
            ctx.async_try_clone().await?.stop().await
        }
        Ok(_) => Ok(()),
    }
}

/// Return the configuration file path
/// This must be the first argument passed to the executable
fn get_configuration_path() -> Result<String> {
    let args: Vec<String> = env::args().collect();
    args.get(1)
        .ok_or(Error::new(
            Origin::Node,
            Kind::Io,
            "Please provide the path to a configuration file",
        ))
        .cloned()
}
