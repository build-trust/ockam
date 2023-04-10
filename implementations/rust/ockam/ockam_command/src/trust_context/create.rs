use crate::{
    util::{
        api::{TrustContextConfigBuilder, TrustContextOpts},
        node_rpc, random_name,
    },
    CommandGlobalOpts,
};
use anyhow::anyhow;
use clap::Args;
use ockam::Context;

#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = false)]
pub struct CreateCommand {
    /// The name of the trust context to create
    #[arg(default_value_t = random_name())]
    name: String,

    /// Create a trust context from a credential
    #[arg(long)]
    credential: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts, trust_context_opts: TrustContextOpts) {
        node_rpc(run_impl, (options, self, trust_context_opts));
    }
}

async fn run_impl(
    _ctx: Context,
    (opts, cmd, tco): (CommandGlobalOpts, CreateCommand, TrustContextOpts),
) -> crate::Result<()> {
    let tcc = TrustContextConfigBuilder::new(&tco)
        .with_credential_name(cmd.credential.as_ref())
        .build();

    if let Some(tcc) = tcc {
        opts.state.trust_contexts.create(&cmd.name, tcc)?;
    } else {
        return Err(anyhow!("Unable to create trust context").into());
    }

    Ok(())
}
