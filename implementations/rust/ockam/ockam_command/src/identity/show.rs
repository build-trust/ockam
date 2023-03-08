use crate::util::output::Output;
use crate::util::print_output;
use crate::{docs, CommandGlobalOpts, EncodeFormat, Result};
use anyhow::anyhow;
use clap::Args;
use core::fmt::Write;
use ockam::identity::identity::IdentityChangeHistory;
use ockam_api::cli_state::CliState;
use ockam_api::nodes::models::identity::{LongIdentityResponse, ShortIdentityResponse};

const LONG_ABOUT: &str = include_str!("./static/show/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show the details of a node
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    #[arg(default_value_t = default_identity_name())]
    name: String,

    #[arg(short, long)]
    full: bool,

    //TODO: see if it make sense to have a --encoding argument shared across commands.
    //      note the only reason this is here right now is that project.json expect the
    //      authority' identity change history to be in hex format.  This only applies
    //      for `full` (change history) identity.
    #[arg(long, value_enum, requires = "full")]
    encoding: Option<EncodeFormat>,
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
        if Some(EncodeFormat::Hex) == cmd.encoding {
            print_output(identity, &opts.global_args.output_format)?;
        } else {
            let output = LongIdentityResponse::new(identity);
            print_output(output, &opts.global_args.output_format)?;
        }
    } else {
        let output = ShortIdentityResponse::new(state.config.identifier.to_string());
        print_output(output, &opts.global_args.output_format)?;
    }
    Ok(())
}

impl Output for LongIdentityResponse<'_> {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        let id: IdentityChangeHistory = serde_bare::from_slice(self.identity.0.as_ref())?;
        write!(w, "{id}")?;
        Ok(w)
    }
}

impl Output for ShortIdentityResponse<'_> {
    fn output(&self) -> Result<String> {
        let mut w = String::new();
        write!(w, "{}", self.identity_id)?;
        Ok(w)
    }
}

fn default_identity_name() -> String {
    let state =
        CliState::try_default().expect("Failed to load CLI state. Try running 'ockam reset'");
    state
        .identities
        .default()
        .map(|i| i.name)
        // Return empty string so we can return a proper error message from the command
        .unwrap_or_else(|_| "".to_string())
}
