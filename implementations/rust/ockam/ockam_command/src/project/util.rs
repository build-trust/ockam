use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use tracing::{debug, trace};

use ockam::identity::IdentityIdentifier;
use ockam::TcpTransport;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::CloudRequestWrapper;
use ockam_api::config::lookup::LookupMeta;
use ockam_api::nodes::models;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelResponse;
use ockam_core::api::Request;
use ockam_multiaddr::{MultiAddr, Protocol};

use crate::util::RpcBuilder;
use crate::CommandGlobalOpts;

pub fn clean_projects_multiaddr(
    input: MultiAddr,
    projects_secure_channels: Vec<MultiAddr>,
) -> Result<MultiAddr> {
    let mut new_ma = MultiAddr::default();
    let mut sc_iter = projects_secure_channels.iter().peekable();
    for p in input.iter().peekable() {
        match p.code() {
            ockam_multiaddr::proto::Project::CODE => {
                let alias = p
                    .cast::<ockam_multiaddr::proto::Project>()
                    .ok_or_else(|| anyhow!("Invalid project value"))?;
                let sc = sc_iter
                    .next()
                    .ok_or_else(|| anyhow!("Missing secure channel for project {}", &*alias))?;
                for v in sc.iter().peekable() {
                    new_ma.push_back_value(&v)?;
                }
            }
            _ => new_ma.push_back_value(&p)?,
        }
    }
    debug!(%input, %new_ma, "Projects names replaced with secure channels");
    Ok(new_ma)
}

pub async fn lookup_projects(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    tcp: &TcpTransport,
    meta: &LookupMeta,
    cloud_addr: &MultiAddr,
    api_node: &str,
) -> Result<Vec<MultiAddr>> {
    let cfg_lookup = opts.config.get_lookup();
    let mut sc = Vec::with_capacity(meta.project.len());
    for name in meta.project.iter() {
        // Try to get the project node's access route + identity id from the config
        let (project_access_route, project_identity_id) = match cfg_lookup.get_project(name) {
            Some(p) => (p.node_route(), p.identity_id.to_string()),
            None => {
                trace!(%name, "Project not found in config, retrieving from cloud");
                // If it's not in the config, retrieve it from the API
                let mut rpc = RpcBuilder::new(ctx, opts, api_node).tcp(tcp).build()?;
                rpc.request(
                    Request::get(format!("v0/projects/name/{}", name))
                        .body(CloudRequestWrapper::bare(cloud_addr)),
                )
                .await?;
                let project = rpc
                    .parse_response::<Project>()
                    .context("Failed to parse Project response")?;
                let identity_id = project
                    .identity
                    .as_ref()
                    .expect("Project should have identity set")
                    .to_string();
                // Store the project in the config lookup table
                opts.config.set_project_alias(
                    project.name.to_string(),
                    project.access_route.to_string(),
                    project.id.to_string(),
                    identity_id.to_string(),
                )?;
                // Return the project data needed to create the secure channel
                (project.access_route(), identity_id)
            }
        };
        // Now we can create the secure channel to the project's node
        sc.push(
            create_secure_channel_to_project(
                ctx,
                opts,
                tcp,
                api_node,
                &project_access_route,
                &project_identity_id,
            )
            .await?,
        );
    }
    // There should be the same number of project occurrences in the
    // input MultiAddr than there are in the secure channels vector.
    assert_eq!(meta.project.len(), sc.len());
    Ok(sc)
}

async fn create_secure_channel_to_project<'a>(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    tcp: &TcpTransport,
    api_node: &str,
    project_access_route: &MultiAddr,
    project_identity: &str,
) -> crate::Result<MultiAddr> {
    let authorized_identifier = vec![IdentityIdentifier::from_str(project_identity)?];
    let mut rpc = RpcBuilder::new(ctx, opts, api_node).tcp(tcp).build()?;
    let req = {
        let payload = models::secure_channel::CreateSecureChannelRequest::new(
            project_access_route,
            Some(authorized_identifier),
        );
        Request::post("/node/secure_channel").body(payload)
    };
    rpc.request(req).await?;
    let sc = rpc.parse_response::<CreateSecureChannelResponse>()?;
    Ok(sc.addr()?)
}
