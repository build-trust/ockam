use anyhow::{anyhow, Context};
use clap::Args;

use crate::IdentityOpts;
use ockam::TcpTransport;
use ockam_api::auth::types::Attributes;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::embedded_node;

#[derive(Clone, Debug, Args)]
pub struct GenerateEnrollmentTokenCommand {
    /// Ockam's cloud address
    #[clap(display_order = 1000)]
    address: MultiAddr,

    #[clap(display_order = 1001, long, default_value = "default")]
    vault: String,

    #[clap(display_order = 1002, long, default_value = "default")]
    identity: String,

    /// Comma-separated list of attributes
    #[clap(last = true, required = true)]
    attributes: Vec<String>,

    #[clap(flatten)]
    identity_opts: IdentityOpts,
}

impl GenerateEnrollmentTokenCommand {
    pub fn run(command: GenerateEnrollmentTokenCommand) {
        embedded_node(generate, command);
    }
}

async fn generate(
    mut ctx: ockam::Context,
    cmd: GenerateEnrollmentTokenCommand,
) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;
    let identifier = identity.identifier()?;

    let route = ockam_api::multiaddr_to_route(&cmd.address)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cmd.address))?;

    let mut attributes = Attributes::new();
    for kv in &cmd.attributes {
        let mut s = kv.split(',');
        let k = s.next().context(format!(
            "failed to parse key from pair: {kv:?}. Expected a \"key,value\" pair."
        ))?;
        let v = s
            .next()
            .context(format!("no value found on pair: {kv:?}"))?;
        if k.is_empty() {
            anyhow::bail!("attribute name can't be empty at pair {kv:?}")
        } else if v.is_empty() {
            anyhow::bail!("attribute value can't be empty at pair {kv:?}")
        } else {
            attributes.put(k, v.as_bytes());
        }
    }

    let mut api_client = ockam_api::cloud::MessagingClient::new(route, &ctx).await?;
    let token = api_client
        .generate_enrollment_token(identifier.key_id().to_string(), attributes)
        .await?;
    println!("Token generated successfully: {:?}", token.token);

    ctx.stop().await?;
    Ok(())
}
