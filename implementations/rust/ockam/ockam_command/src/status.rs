use std::fmt::Write;
use std::time::Duration;

use clap::Args;
use miette::IntoDiagnostic;
use tracing::warn;

use ockam::identity::{Identifier, TimestampInSeconds};
use ockam::Context;
use ockam_api::cli_state::{EnrollmentStatus, IdentityEnrollment, NodeInfo};
use ockam_api::cloud::project::models::OrchestratorVersionInfo;
use ockam_api::nodes::models::node::NodeStatus;
use ockam_api::nodes::InMemoryNode;

use crate::util::{async_cmd, duration::duration_parser};
use crate::CommandGlobalOpts;
use crate::Result;

/// Display information about the system's status
#[derive(Clone, Debug, Args)]
pub struct StatusCommand {
    /// Show status for all identities; default: enrolled only
    #[arg(long, short)]
    all: bool,

    /// Override the default timeout
    #[arg(long, default_value = "5", value_parser = duration_parser)]
    timeout: Duration,
}

impl StatusCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "status".to_string()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let identities_details = get_identities_details(&opts, self.all).await?;
        let nodes = opts.state.get_nodes().await?;
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
            .plain(build_plain_output(self, &status).await?)
            .json(serde_json::to_string(&status).into_diagnostic()?)
            .write_line()?;
        Ok(())
    }
}

async fn get_identities_details(
    opts: &CommandGlobalOpts,
    all: bool,
) -> Result<Vec<IdentityEnrollment>> {
    let enrollment_status = if all {
        EnrollmentStatus::Any
    } else {
        EnrollmentStatus::Enrolled
    };
    Ok(opts
        .state
        .get_identity_enrollments(enrollment_status)
        .await?)
}

async fn build_plain_output(cmd: &StatusCommand, status: &StatusData) -> Result<String> {
    let mut plain = String::new();
    writeln!(
        plain,
        "Controller version: {}",
        status.orchestrator_version.version()
    )?;
    writeln!(
        plain,
        "Project version: {}",
        status.orchestrator_version.project_version()
    )?;
    if status.identities.is_empty() {
        if cmd.all {
            writeln!(plain, "No identities found")?;
        } else {
            writeln!(
                &mut plain,
                "No enrolled identities could be found. \
                Try passing the `--all` argument to see all identities, and not just the enrolled ones. \
                Also consider running `ockam enroll` to enroll an identity.",
            )?;
        }
        return Ok(plain);
    };

    for (i_idx, i) in status.identities.iter().enumerate() {
        writeln!(plain, "Identity[{i_idx}]")?;
        if i.is_default() {
            writeln!(plain, "{:2}Default: yes", "")?;
        }
        if let Some(name) = i.name() {
            writeln!(&mut plain, "{:2}Name: {}", "", name)?;
        }
        writeln!(plain, "{:2}Identifier: {}", "", i.identifier())?;
        writeln!(plain, "{:2}Enrolled: {}", "", i.is_enrolled())?;

        if !i.nodes.is_empty() {
            writeln!(plain, "{:2}Linked Nodes:", "")?;
            for (n_idx, node) in i.nodes.iter().enumerate() {
                writeln!(plain, "{:4}Node[{}]:", "", n_idx)?;
                writeln!(plain, "{:6}Name: {}", "", node.name)?;
                writeln!(plain, "{:6}Status: {:?}", "", node.status)?;
            }
        }
    }
    Ok(plain)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StatusData {
    #[serde(flatten)]
    orchestrator_version: OrchestratorVersionInfo,
    identities: Vec<IdentityWithLinkedNodes>,
}

impl StatusData {
    fn from_parts(
        orchestrator_version: OrchestratorVersionInfo,
        identities_details: Vec<IdentityEnrollment>,
        nodes: Vec<NodeInfo>,
    ) -> Result<Self> {
        let mut identities = vec![];
        for identity in identities_details.into_iter() {
            let identity_status = IdentityWithLinkedNodes {
                identifier: identity.identifier().clone(),
                name: identity.name().clone(),
                is_default: identity.is_default(),
                enrolled_at: identity
                    .enrolled_at()
                    .map(|o| TimestampInSeconds::from(o.unix_timestamp() as u64)),
                nodes: nodes
                    .iter()
                    .filter_map(|node| {
                        if node.identifier() == identity.identifier().clone() {
                            Some(NodeStatus::from(node))
                        } else {
                            None
                        }
                    })
                    .collect(),
            };
            identities.push(identity_status);
        }
        Ok(Self {
            orchestrator_version,
            identities,
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct IdentityWithLinkedNodes {
    identifier: Identifier,
    name: Option<String>,
    is_default: bool,
    enrolled_at: Option<TimestampInSeconds>,
    nodes: Vec<NodeStatus>,
}

impl IdentityWithLinkedNodes {
    fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }

    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn is_default(&self) -> bool {
        self.is_default
    }

    fn is_enrolled(&self) -> bool {
        self.enrolled_at.is_some()
    }
}
