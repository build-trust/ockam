use crate::util::output::Output;
use crate::util::print_output;
use crate::CommandGlobalOpts;
use anyhow::anyhow;
use clap::Args;
use core::fmt::Write;
use ockam_api::cli_state::CliState;
use ockam_api::nodes::models::identity::{LongIdentityResponse, ShortIdentityResponse};
use ockam_identity::change_history::IdentityChangeHistory;

#[derive(Clone, Debug, Args)]
pub struct ShowCommand {
    #[arg(default_value_t = default_identity_name())]
    name: String,
    #[arg(short, long)]
    full: bool,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(options, self) {
            eprintln!("{e:?}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: ShowCommand) -> crate::Result<()> {
    if cmd.name.is_empty() {
        return Err(
            anyhow!("Default identity not found. Have you run 'ockam identity create'?").into(),
        );
    }
    let state = opts.state.identities.get(&cmd.name)?;
    if cmd.full {
        let identity = state.config.change_history.export()?;
        let output = LongIdentityResponse::new(identity);
        print_output(output, &opts.global_args.output_format)?;
    } else {
        let output = ShortIdentityResponse::new(state.config.identifier.to_string());
        print_output(output, &opts.global_args.output_format)?;
    }
    Ok(())
}

impl Output for LongIdentityResponse<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        let id: IdentityChangeHistory = serde_bare::from_slice(self.identity.0.as_ref())?;
        write!(w, "{}", id)?;
        Ok(w)
    }
}

impl Output for ShortIdentityResponse<'_> {
    fn output(&self) -> anyhow::Result<String> {
        let mut w = String::new();
        write!(w, "{}", self.identity_id)?;
        Ok(w)
    }
}

fn default_identity_name() -> String {
    let state = CliState::new().expect("Failed to load CLI state. Try running 'ockam reset'");
    state
        .identities
        .default()
        .map(|i| i.name)
        // Return empty string so we can return a proper error message from the command
        .unwrap_or_else(|_| "".to_string())
}
