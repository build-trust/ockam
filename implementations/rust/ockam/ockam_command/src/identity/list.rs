use crate::util::exitcode;
use crate::util::output::Output;
use crate::CommandGlobalOpts;
use anyhow::anyhow;
use clap::Args;
use ockam_api::nodes::models::identity::{LongIdentityResponse, ShortIdentityResponse};

/// List nodes
#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[arg(short, long)]
    full: bool,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if let Err(e) = run_impl(options, self) {
            eprintln!("{e:?}");
            std::process::exit(e.code());
        }
    }
}

fn run_impl(opts: CommandGlobalOpts, cmd: ListCommand) -> crate::Result<()> {
    let idts = opts.state.identities.list()?;
    if idts.is_empty() {
        return Err(crate::Error::new(
            exitcode::IOERR,
            anyhow!("No identities registered on this system!"),
        ));
    }
    for (idx, identity) in idts.iter().enumerate() {
        let state = opts.state.identities.get(&identity.name)?;
        let default = if opts.state.identities.default()?.name == identity.name {
            " (default)"
        } else {
            ""
        };
        println!("Identity[{idx}]:");
        println!("{:2}Name: {}{}", "", &identity.name, default);
        if cmd.full {
            let identity = state.config.change_history.export()?;
            let output = LongIdentityResponse::new(identity);
            println!("{:2}{}", "", &output.output()?);
        } else {
            let output = ShortIdentityResponse::new(state.config.identifier.to_string());
            println!("{:2}Identifier: {}", "", &output.output()?);
        };
        if idx < idts.len() - 1 {
            println!();
        }
    }
    Ok(())
}
