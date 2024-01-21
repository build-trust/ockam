use std::fmt::Write;
use std::time::Duration;

use clap::Args;
use miette::IntoDiagnostic;
use tracing::warn;

use ockam::identity::{Identifier, TimestampInSeconds};
use ockam::Context;
use ockam_api::cli_state::{EnrollmentStatus, IdentityEnrollment};
use ockam_api::cloud::project::OrchestratorVersionInfo;
use ockam_api::nodes::models::base::NodeStatus as NodeStatusModel;
use ockam_api::nodes::{BackgroundNodeClient, InMemoryNode};

use crate::util::{api, duration::duration_parser, node_rpc};
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
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, StatusCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: StatusCommand,
) -> miette::Result<()> {
    let identities_details = get_identities_details(&opts, cmd.all).await?;
    let nodes_details = get_nodes_details(ctx, &opts).await?;

    let node = InMemoryNode::start(ctx, &opts.state)
        .await?
        .with_timeout(cmd.timeout);
    let controller = node.create_controller().await?;
    let orchestrator_version = controller
        .get_orchestrator_version_info(ctx)
        .await
        .map_err(|e| warn!(%e, "Failed to retrieve orchestrator version"))
        .unwrap_or_default();

    let status = StatusData::from_parts(orchestrator_version, identities_details, nodes_details)?;
    opts.terminal
        .stdout()
        .plain(build_plain_output(&cmd, &status).await?)
        .json(serde_json::to_string(&status).into_diagnostic()?)
        .write_line()?;
    Ok(())
}

async fn get_nodes_details(ctx: &Context, opts: &CommandGlobalOpts) -> Result<Vec<NodeDetails>> {
    let mut node_details: Vec<NodeDetails> = vec![];

    let nodes = opts.state.get_nodes().await?;
    if nodes.is_empty() {
        return Ok(node_details);
    }
    let default_node_name = opts.state.get_default_node().await?.name();
    let mut node_client =
        BackgroundNodeClient::create_to_node(ctx, &opts.state, &default_node_name).await?;
    node_client.set_timeout(Duration::from_millis(200));

    for node in nodes {
        node_client.set_node_name(&node.name());
        let node_infos = NodeDetails {
            identifier: node.identifier(),
            name: node.name(),
            status: get_node_status(ctx, &node_client).await?,
        };
        node_details.push(node_infos);
    }

    Ok(node_details)
}

async fn get_node_status(ctx: &Context, node: &BackgroundNodeClient) -> Result<String> {
    let node_status_model: miette::Result<NodeStatusModel> =
        node.ask(ctx, api::query_status()).await;
    Ok(node_status_model
        .map(|m| m.status)
        .unwrap_or("Stopped".to_string()))
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
                writeln!(plain, "{:6}Status: {}", "", node.status)?;
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
        mut nodes_details: Vec<NodeDetails>,
    ) -> Result<Self> {
        let mut identities = vec![];
        for identity in identities_details.into_iter() {
            let mut identity_status = IdentityWithLinkedNodes {
                identifier: identity.identifier(),
                name: identity.name(),
                is_default: identity.is_default(),
                enrolled_at: identity
                    .enrolled_at()
                    .map(|o| TimestampInSeconds::from(o.unix_timestamp() as u64)),
                nodes: vec![],
            };
            nodes_details.retain(|nd| nd.identifier == identity_status.identifier());
            if !nodes_details.is_empty() {
                identity_status.nodes = nodes_details.clone();
            }
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
    nodes: Vec<NodeDetails>,
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

    #[allow(unused)]
    fn nodes(&self) -> &Vec<NodeDetails> {
        &self.nodes
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct NodeDetails {
    identifier: Identifier,
    name: String,
    status: String,
}
