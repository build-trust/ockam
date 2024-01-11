use crate::run::parser::{ArgsToCommands, Config};
use crate::CommandGlobalOpts;

use miette::Result;
use ockam_node::Context;

/// Parses a given configuration and runs the necessary commands to create the described state.
///
/// More specifically, this struct is responsible for:
/// - Running the commands in a valid order. For example, nodes will be created before TCP inlets.
/// - Do the necessary checks to to run only the necessary commands. For example, an enrollment ticket won't
///  be used if the identity is already enrolled.
///
/// For more details about the parsing, see the [Config](Config) struct. You can also check examples of
/// valid configuration files in the demo folder of this module.
pub struct ConfigRunner;

impl ConfigRunner {
    pub async fn run_config(
        ctx: &Context,
        opts: CommandGlobalOpts,
        config_contents: &str,
    ) -> Result<()> {
        let config = Config::parse(config_contents)?;

        let projects = config.projects.into_commands()?;
        for project_enrollment in projects.enroll {
            let identity_name = &project_enrollment.cloud_opts.identity;
            if let Ok(is_enrolled) = opts.state.is_identity_enrolled(identity_name).await {
                if is_enrolled {
                    continue;
                }
            }
            project_enrollment.async_run(ctx, opts.clone()).await?;
        }

        let nodes = config.nodes.into_commands()?;
        for node in nodes {
            node.async_run(ctx, opts.clone(), None).await?;
        }

        let relays = config.relays.into_commands()?;
        for relay in relays {
            relay.async_run(ctx, opts.clone()).await?;
        }

        let policies = config.policies.into_commands()?;
        for policy in policies {
            policy.async_run(ctx, opts.clone()).await?;
        }

        let tcp_outlets = config.tcp_outlets.into_commands()?;
        for tcp_outlet in tcp_outlets {
            tcp_outlet.async_run(ctx, opts.clone()).await?;
        }

        let tcp_inlets = config.tcp_inlets.into_commands()?;
        for tcp_inlet in tcp_inlets {
            tcp_inlet.async_run(ctx, opts.clone()).await?;
        }

        Ok(())
    }
}
