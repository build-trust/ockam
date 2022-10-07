use crate::util::{connect_to, exitcode, verify_pids};
use crate::{help, node::show::print_query_status, node::HELP_DETAIL, CommandGlobalOpts};
use clap::Args;

/// List Nodes
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
pub struct ListCommand {}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        let cfg = &options.config;
        let node_names = {
            let inner = cfg.inner();

            if inner.nodes.is_empty() {
                println!("No nodes registered on this system!");
                std::process::exit(exitcode::IOERR);
            }

            // Before printing node state we have to verify it.  This
            // happens by sending a QueryStatus request to every node on
            // record.  If the function fails, then it is assumed not to
            // be up.  Also, if the function returns, but yields a
            // different pid, then we update the pid stored in the config.
            // This should only happen if the node has failed in the past,
            // and has been restarted by something that is not this CLI.
            inner.nodes.iter().map(|(name, _)| name.clone()).collect()
        };
        verify_pids(cfg, node_names);

        cfg.inner().nodes.iter().for_each(|(node_name, node_cfg)| {
            connect_to(
                node_cfg.port(),
                (cfg.clone(), node_name.clone(), false),
                print_query_status,
            )
        });
    }
}
