use anyhow::anyhow;
use clap::Args;

use ockam::TcpTransport;
use ockam_api::auth::types::Attributes;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::embedded_node;
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct GenerateEnrollmentTokenCommand {
    /// Ockam's cloud address
    #[clap(display_order = 1000)]
    address: MultiAddr,

    #[clap(display_order = 1001, long, default_value = "default")]
    vault: String,

    #[clap(display_order = 1002, long, default_value = "default")]
    identity: String,

    /// Attributes (use '=' to separate key from value)
    #[clap(value_delimiter('='), last = true, required = true)]
    attrs: Vec<String>,

    #[clap(flatten)]
    identity_opts: IdentityOpts,
}

impl GenerateEnrollmentTokenCommand {
    pub fn run(cmd: GenerateEnrollmentTokenCommand) {
        embedded_node(generate, cmd);
    }
}

async fn generate(
    mut ctx: ockam::Context,
    cmd: GenerateEnrollmentTokenCommand,
) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;

    let route = ockam_api::multiaddr_to_route(&cmd.address)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cmd.address))?;

    let mut attributes = Attributes::new();
    for entry in cmd.attrs.chunks(2) {
        if let [k, v] = entry {
            attributes.put(k, v.as_bytes());
        } else {
            return Err(anyhow!("{entry:?} is not a key-value pair"));
        }
    }

    let mut api_client = ockam_api::cloud::MessagingClient::new(route, identity, &ctx).await?;
    let token = api_client.generate_enrollment_token(attributes).await?;
    println!("Token generated successfully: {:?}", token.token);

    ctx.stop().await?;
    Ok(())
}
