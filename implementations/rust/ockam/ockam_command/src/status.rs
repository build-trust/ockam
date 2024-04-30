use async_trait::async_trait;
use std::fmt::Display;
use std::time::Duration;

use clap::Args;
use colorful::Colorful;
use miette::IntoDiagnostic;
use serde::Serialize;
use tracing::warn;

use ockam::Context;
use ockam_api::cli_state::{EnrollmentFilter, IdentityEnrollment};
use ockam_api::cloud::project::models::OrchestratorVersionInfo;
use ockam_api::colors::color_primary;
use ockam_api::nodes::models::node::NodeResources;
use ockam_api::nodes::{BackgroundNodeClient, InMemoryNode};
use ockam_api::{fmt_heading, fmt_log, fmt_separator, fmt_warn};

use crate::node::show::get_node_resources;
use crate::util::duration::duration_parser;
use crate::Result;
use crate::{Command, CommandGlobalOpts};

/// Display information about the system's status
#[derive(Clone, Debug, Args)]
pub struct StatusCommand {
    /// Override the default timeout
    #[arg(long, default_value = "5", value_parser = duration_parser)]
    timeout: Duration,
}

#[async_trait]
impl Command for StatusCommand {
    const NAME: &'static str = "status";

    async fn async_run(self, ctx: &Context, opts: CommandGlobalOpts) -> Result<()> {
        let identities_details = self.get_identities_details(&opts).await?;
        let nodes = self.get_nodes_resources(ctx, &opts).await?;
        let orchestrator_version = {
            let node = InMemoryNode::start(ctx, &opts.state)
                .await?
                .with_timeout(self.timeout);
            let controller = node.create_controller().await?;
            controller
                .get_orchestrator_version_info(ctx)
                .await
                .map_err(|e| warn!(%e, "Failed to retrieve orchestrator version"))
                .unwrap_or_default()
        };
        let status = StatusData::from_parts(orchestrator_version, identities_details, nodes)?;
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
        let pb = opts.terminal.progress_spinner();
        let nodes = opts.state.get_nodes().await?;
        for node in nodes {
            if let Some(ref pb) = pb {
                pb.set_message(format!("Retrieving node {}...", node.name()));
            }
            let mut node =
                BackgroundNodeClient::create(ctx, &opts.state, &Some(node.name())).await?;
            nodes_resources.push(get_node_resources(ctx, &opts.state, &mut node, false).await?);
        }
        if let Some(ref pb) = pb {
            pb.finish_and_clear();
        }
        Ok(nodes_resources)
    }
}

#[derive(Serialize)]
struct StatusData {
    #[serde(flatten)]
    orchestrator_version: OrchestratorVersionInfo,
    identities: Vec<IdentityEnrollment>,
    nodes: Vec<NodeResources>,
}

impl StatusData {
    fn from_parts(
        orchestrator_version: OrchestratorVersionInfo,
        identities: Vec<IdentityEnrollment>,
        nodes: Vec<NodeResources>,
    ) -> Result<Self> {
        Ok(Self {
            orchestrator_version,
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
