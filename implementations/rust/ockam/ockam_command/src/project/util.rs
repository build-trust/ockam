use std::io::Write;
use std::str::FromStr;

use anyhow::{anyhow, Context as _, Result};
use tracing::debug;

use ockam::identity::IdentityIdentifier;
use ockam::TcpTransport;
use ockam_api::cloud::project::Project;
use ockam_api::config::lookup::LookupMeta;
use ockam_api::nodes::models;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelResponse;
use ockam_core::api::Request;
use ockam_multiaddr::{MultiAddr, Protocol};

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::{api, RpcBuilder};
use crate::{CommandGlobalOpts, OckamConfig};

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

pub async fn get_projects_secure_channels_from_config_lookup(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    tcp: &TcpTransport,
    meta: &LookupMeta,
    cloud_addr: &MultiAddr,
    api_node: &str,
) -> Result<Vec<MultiAddr>> {
    let cfg_lookup = opts.config.get_lookup();
    let mut sc = Vec::with_capacity(meta.project.len());

    // In case a project is missing from the config file, we fetch them all from the cloud.
    let missing_projects = meta
        .project
        .iter()
        .any(|name| cfg_lookup.get_project(name).is_none());
    if missing_projects {
        config::refresh_projects(ctx, opts, tcp, api_node, cloud_addr).await?;
    }

    // Create a secure channel for each project.
    for name in meta.project.iter() {
        // Get the project node's access route + identity id from the config
        let (project_access_route, project_identity_id) = {
            // This shouldn't fail, as we did a refresh above if we found any missing project.
            let p = cfg_lookup
                .get_project(name)
                .context(format!("Failed to get project {} from config lookup", name))?;
            (p.node_route(), p.identity_id.to_string())
        };
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

pub async fn check_project_readiness<'a>(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    node_opts: &NodeOpts,
    cloud_opts: &CloudOpts,
    tcp: &TcpTransport,
    mut project: Project<'a>,
) -> Result<Project<'a>> {
    if !project.is_ready() {
        print!("\nProject created. Waiting until it's operative...");
        let cloud_route = cloud_opts.route();
        loop {
            print!(".");
            std::io::stdout().flush()?;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let mut rpc = RpcBuilder::new(ctx, opts, &node_opts.api_node)
                .tcp(tcp)
                .build()?;
            rpc.request(api::project::show(&project.id, cloud_route))
                .await?;
            let p = rpc.parse_response::<Project>()?;
            if p.is_ready() {
                project = p.to_owned();
                break;
            }
        }
    }
    if !project.is_reachable().await? {
        print!("\nEstablishing connection (this can take a few minutes)...");
        loop {
            print!(".");
            std::io::stdout().flush()?;
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            if project.is_reachable().await? {
                std::io::stdout().flush()?;
                break;
            }
        }
        println!();
    }
    Ok(project)
}

pub mod config {
    use super::*;
    use ockam::Context;

    pub fn set_project(config: &OckamConfig, project: &Project) -> Result<()> {
        config.set_project_alias(
            project.name.to_string(),
            project.access_route.to_string(),
            project.id.to_string(),
            project
                .identity
                .as_ref()
                .expect("Project should have identity set")
                .to_string(),
        )?;
        config.atomic_update().run()?;
        Ok(())
    }

    pub fn set_projects(config: &OckamConfig, projects: &[Project]) -> Result<()> {
        config.remove_projects_alias();
        for project in projects.iter() {
            config.set_project_alias(
                project.name.to_string(),
                project.access_route.to_string(),
                project.id.to_string(),
                project
                    .identity
                    .as_ref()
                    .expect("Project should have identity set")
                    .to_string(),
            )?;
        }
        config.atomic_update().run()?;
        Ok(())
    }

    pub fn remove_project(config: &OckamConfig, name: &str) -> Result<()> {
        config.remove_project_alias(name)?;
        config.atomic_update().run()?;
        Ok(())
    }

    pub fn get_project(config: &OckamConfig, name: &str) -> Option<String> {
        let inner = config.writelock_inner();
        inner.lookup.get_project(name).map(|s| s.id.clone())
    }

    pub async fn refresh_projects(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        tcp: &TcpTransport,
        api_node: &str,
        controller_route: &MultiAddr,
    ) -> Result<()> {
        let mut rpc = RpcBuilder::new(ctx, opts, api_node).tcp(tcp).build()?;
        rpc.request(api::project::list(controller_route)).await?;
        let projects = rpc.parse_response::<Vec<Project>>()?;
        set_projects(&opts.config, &projects)?;
        Ok(())
    }
}
