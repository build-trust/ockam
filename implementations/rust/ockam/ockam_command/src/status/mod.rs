use async_trait::async_trait;
use std::fmt::Display;

use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use serde::Serialize;
use tracing::warn;

use crate::node::show::get_node_resources;
use crate::shared_args::TimeoutArg;
use crate::version::Version;
use crate::Result;
use crate::{Command, CommandGlobalOpts};
use ockam::Context;
use ockam_api::cli_state::{EnrollmentFilter, IdentityEnrollment};
use ockam_api::colors::color_primary;
use ockam_api::nodes::models::node::NodeResources;
use ockam_api::nodes::{BackgroundNodeClient, InMemoryNode};
use ockam_api::orchestrator::project::models::OrchestratorVersionInfo;
use ockam_api::orchestrator::space::Space;
use ockam_api::output::Output;
use ockam_api::{fmt_heading, fmt_log, fmt_separator, fmt_warn};

use crate::docs;

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

#[derive(Clone, Debug, Args)]
#[command(
about = docs::about("Display information about your Ockam instance"),
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct StatusCommand {
    #[command(flatten)]
    timeout: TimeoutArg,
}

#[async_trait]
impl Command for StatusCommand {
    const NAME: &'static str = "status";

    async fn run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let identities_details = self.get_identities_details(&opts).await?;
        let nodes = self.get_nodes_resources(ctx, &opts).await?;
        let node = InMemoryNode::start(ctx, &opts.state)
            .await?
            .with_timeout(self.timeout.timeout);
        let controller = node.create_controller().await?;
        let orchestrator_version = controller
            .get_orchestrator_version_info(ctx)
            .await
            .map_err(|e| warn!(%e, "Failed to retrieve orchestrator version"))
            .unwrap_or_default();
        let spaces = opts.state.get_spaces().await?;
        let status =
            StatusData::from_parts(orchestrator_version, spaces, identities_details, nodes)?;
        opts.terminal
            .stdout()
            .plain(&status)
            .json(serde_json::to_string(&status).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}

impl StatusCommand {
    async fn get_identities_details(
        &self,
        opts: &CommandGlobalOpts,
    ) -> Result<Vec<IdentityEnrollment>> {
        Ok(opts
            .state
            .get_identity_enrollments(EnrollmentFilter::Any)
            .await?)
    }

    async fn get_nodes_resources(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
    ) -> Result<Vec<NodeResources>> {
        let mut nodes_resources = vec![];
        let pb = opts.terminal.spinner();
        let nodes = opts.state.get_nodes().await?;
        for node in nodes {
            if let Some(ref pb) = pb {
                pb.set_message(format!("Retrieving node {}...", node.name()));
            }
            let mut node =
                BackgroundNodeClient::create(ctx, &opts.state, &Some(node.name())).await?;
            nodes_resources.push(get_node_resources(ctx, &opts.state, &mut node).await?);
        }
        if let Some(ref pb) = pb {
            pb.finish_and_clear();
        }
        Ok(nodes_resources)
    }
}

#[derive(Serialize)]
struct StatusData {
    ockam_version: Version,
    orchestrator_version: OrchestratorVersionInfo,
    spaces: Vec<Space>,
    identities: Vec<IdentityEnrollment>,
    nodes: Vec<NodeResources>,
}

impl StatusData {
    fn from_parts(
        orchestrator_version: OrchestratorVersionInfo,
        spaces: Vec<Space>,
        identities: Vec<IdentityEnrollment>,
        nodes: Vec<NodeResources>,
    ) -> Result<Self> {
        Ok(Self {
            ockam_version: Version::new(),
            orchestrator_version,
            spaces,
            identities,
            nodes,
        })
    }
}

impl Display for StatusData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}",
            fmt_log!(
                "Ockam version: {}",
                color_primary(self.ockam_version.to_string())
            )
        )?;
        writeln!(
            f,
            "{}",
            fmt_log!(
                "Controller version: {}",
                color_primary(self.orchestrator_version.version())
            )
        )?;
        writeln!(
            f,
            "{}",
            fmt_log!(
                "Project version: {}",
                color_primary(self.orchestrator_version.project_version())
            )
        )?;

        if self.spaces.is_empty() {
            writeln!(f, "{}", fmt_separator!())?;
            writeln!(f, "{}", fmt_warn!("No spaces found"))?;
            writeln!(
                f,
                "{}",
                fmt_log!("Consider running `ockam enroll` or `ockam space create` to create your first space.")
            )?;
        } else {
            writeln!(f, "{}", fmt_heading!("Spaces"))?;
            for (idx, space) in self.spaces.iter().enumerate() {
                if idx > 0 {
                    writeln!(f)?;
                }
                writeln!(f, "{}", space.iter_output().pad())?;
            }
        }

        if self.identities.is_empty() {
            writeln!(f, "{}", fmt_separator!())?;
            writeln!(f, "{}", fmt_warn!("No identities found"))?;
            writeln!(
                f,
                "{}",
                fmt_log!("Consider running `ockam enroll` to enroll an identity.")
            )?;
        } else {
            writeln!(f, "{}", fmt_heading!("Identities"))?;
            for (idx, i) in self.identities.iter().enumerate() {
                if idx > 0 {
                    writeln!(f)?;
                }
                write!(f, "{}", i)?;
            }
        }

        if self.nodes.is_empty() {
            writeln!(f, "{}", fmt_separator!())?;
            writeln!(f, "{}", fmt_warn!("No nodes found"))?;
            writeln!(
                f,
                "{}",
                fmt_log!("Consider running `ockam node create` to create your first node.")
            )?;
        } else {
            writeln!(f, "{}", fmt_heading!("Nodes"))?;
            for (idx, node) in self.nodes.iter().enumerate() {
                if idx > 0 {
                    writeln!(f)?;
                }
                write!(f, "{}", node)?;
            }
        }

        Ok(())
    }
}
