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
use ockam_api::cli_state::StateDirTrait;

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
    let tcc = TrustContextConfigBuilder::new(&opts.state, &tco)?
        .with_credential_name(cmd.credential.as_ref())
        .use_default_trust_context(false)
        .build();

    if let Some(tcc) = tcc {
        opts.state.trust_contexts.create(&cmd.name, tcc.clone())?;

        let auth = if let Ok(auth) = tcc.authority() {
            auth.identity_str()
        } else {
            "None"
        };

        let output = format!(
            r#"
Trust Context:
    Name: {}
    ID: {}
    Authority: {}
"#,
            cmd.name,
            tcc.id(),
            auth
        );

        opts.terminal
            .stdout()
            .plain(output)
            .json(serde_json::to_string_pretty(&tcc)?)
            .write_line()?;
    } else {
        return Err(anyhow!("Unable to create trust context").into());
    }

    Ok(())
}
