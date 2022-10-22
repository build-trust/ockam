use std::io::Write;
use std::str::FromStr;

use anyhow::{anyhow, Context as _, Result};
use tracing::debug;

use ockam::identity::IdentityIdentifier;
use ockam::TcpTransport;
use ockam_api::cloud::project::Project;
use ockam_api::config::lookup::{LookupMeta, ProjectAuthority, ProjectLookup};
use ockam_api::multiaddr_to_addr;
use ockam_api::nodes::models::secure_channel::*;
use ockam_multiaddr::{MultiAddr, Protocol};

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
    meta: &LookupMeta,
    cloud_addr: &MultiAddr,
    api_node: &str,
    tcp: Option<&TcpTransport>,
    credential_exchange_mode: CredentialExchangeMode,
) -> Result<Vec<MultiAddr>> {
    let cfg_lookup = opts.config.lookup();
    let mut sc = Vec::with_capacity(meta.project.len());

    // In case a project is missing from the config file, we fetch them all from the cloud.
    if cfg_lookup.has_unresolved_projects(meta) {
        config::refresh_projects(ctx, opts, api_node, cloud_addr, tcp).await?;
    }

    // Create a secure channel for each project.
    for name in meta.project.iter() {
        // Get the project node's access route + identity id from the config
        let (project_access_route, project_identity_id) = {
            // This shouldn't fail, as we did a refresh above if we found any missing project.
            let p = cfg_lookup
                .get_project(name)
                .context(format!("Failed to get project {} from config lookup", name))?;
            let id = p
                .identity_id
                .as_ref()
                .context("Project should have identity set")?;
            let node_route = p
                .node_route
                .as_ref()
                .context("Invalid project node route")?;
            (node_route, id.to_string())
        };
        sc.push(
            create_secure_channel_to_project(
                ctx,
                opts,
                api_node,
                tcp,
                project_access_route,
                &project_identity_id,
                credential_exchange_mode,
            )
            .await?,
        );
    }

    // There should be the same number of project occurrences in the
    // input MultiAddr than there are in the secure channels vector.
    assert_eq!(meta.project.len(), sc.len());
    Ok(sc)
}

async fn create_secure_channel_to_project(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    api_node: &str,
    tcp: Option<&TcpTransport>,
    project_access_route: &MultiAddr,
    project_identity: &str,
    credential_exchange_mode: CredentialExchangeMode,
) -> crate::Result<MultiAddr> {
    let authorized_identifier = vec![IdentityIdentifier::from_str(project_identity)?];
    let mut rpc = RpcBuilder::new(ctx, opts, api_node).tcp(tcp)?.build();
    rpc.request(api::create_secure_channel(
        project_access_route,
        Some(authorized_identifier),
        credential_exchange_mode,
    ))
    .await?;
    let sc = rpc.parse_response::<CreateSecureChannelResponse>()?;
    Ok(sc.addr()?)
}

pub async fn create_secure_channel_to_authority(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    authority: &ProjectAuthority,
    addr: &MultiAddr,
) -> crate::Result<MultiAddr> {
    let mut rpc = RpcBuilder::new(ctx, opts, node_name).build();
    debug!(%addr, "establishing secure channel to project authority");
    let allowed = vec![authority.identity_id().clone()];
    rpc.request(api::create_secure_channel(
        addr,
        Some(allowed),
        CredentialExchangeMode::None,
    ))
    .await?;
    let res = rpc.parse_response::<CreateSecureChannelResponse>()?;
    let addr = res.addr()?;
    Ok(addr)
}

async fn delete_secure_channel<'a>(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    api_node: &str,
    tcp: Option<&TcpTransport>,
    sc_addr: &MultiAddr,
) -> crate::Result<()> {
    let mut rpc = RpcBuilder::new(ctx, opts, api_node).tcp(tcp)?.build();
    let addr = multiaddr_to_addr(sc_addr).context("Failed to convert MultiAddr to addr")?;
    rpc.request(api::delete_secure_channel(&addr)).await?;
    rpc.is_ok()?;
    Ok(())
}

pub async fn check_project_readiness<'a>(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    cloud_opts: &CloudOpts,
    api_node: &str,
    tcp: Option<&TcpTransport>,
    mut project: Project<'a>,
) -> Result<Project<'a>> {
    // Persist project config prior to checking readiness which might take a while
    config::set_project_id(&opts.config, &project).await?;

    if !project.is_ready() {
        print!("\nProject created. Waiting until it's operative...");
        let cloud_route = &cloud_opts.route();
        loop {
            print!(".");
            std::io::stdout().flush()?;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let mut rpc = RpcBuilder::new(ctx, opts, api_node).build();
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
                break;
            }
        }
    }
    {
        print!("\nEstablishing secure channel...");
        std::io::stdout().flush()?;
        let project_route = project.access_route()?;
        let project_identity = project
            .identity
            .as_ref()
            .context("We already checked that the project has an identity")?
            .to_string();
        match create_secure_channel_to_project(
            ctx,
            opts,
            api_node,
            tcp,
            &project_route,
            &project_identity,
            CredentialExchangeMode::None,
        )
        .await
        {
            Ok(sc_addr) => {
                // Try to delete secure channel, ignore result.
                let _ = delete_secure_channel(ctx, opts, api_node, tcp, &sc_addr).await;
            }
            Err(_) => {
                loop {
                    print!(".");
                    std::io::stdout().flush()?;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if let Ok(sc_addr) = create_secure_channel_to_project(
                        ctx,
                        opts,
                        api_node,
                        tcp,
                        &project_route,
                        &project_identity,
                        CredentialExchangeMode::None,
                    )
                    .await
                    {
                        // Try to delete secure channel, ignore result.
                        let _ = delete_secure_channel(ctx, opts, api_node, tcp, &sc_addr).await;
                        break;
                    }
                }
            }
        }
        println!();
    }
    std::io::stdout().flush()?;
    // Persist project config with all its fields
    config::set_project(&opts.config, &project).await?;
    Ok(project)
}

pub mod config {
    use crate::util::output::Output;
    use ockam::Context;
    use ockam_api::cloud::project::OktaAuth0;
    use ockam_api::config::lookup::ProjectAuthority;
    use tracing::trace;

    use super::*;

    async fn set(config: &OckamConfig, project: &Project<'_>) -> Result<()> {
        if !project.is_ready() {
            trace!("Project is not ready yet {}", project.output()?);
            return Err(anyhow!(
                "Project is not ready yet, wait a few seconds and try again"
            ));
        }
        let node_route: MultiAddr = project
            .access_route
            .as_ref()
            .try_into()
            .context("Invalid project node route")?;
        let pid = project
            .identity
            .as_ref()
            .context("Project should have identity set")?;
        let authority = ProjectAuthority::from_raw(
            &project.authority_access_route,
            &project.authority_identity,
        )
        .await?;
        let okta = project.okta_config.as_ref().map(|o| OktaAuth0 {
            tenant: o.tenant.to_string(),
            client_id: o.client_id.to_string(),
        });
        config.set_project_alias(
            project.name.to_string(),
            ProjectLookup {
                node_route: Some(node_route),
                id: project.id.to_string(),
                identity_id: Some(pid.clone()),
                authority,
                okta,
            },
        )?;
        Ok(())
    }

    pub(super) async fn set_project_id(config: &OckamConfig, project: &Project<'_>) -> Result<()> {
        config.set_project_alias(
            project.name.to_string(),
            ProjectLookup {
                node_route: None,
                id: project.id.to_string(),
                identity_id: None,
                authority: None,
                okta: None,
            },
        )?;
        config.persist_config_updates()?;
        Ok(())
    }

    pub async fn set_project(config: &OckamConfig, project: &Project<'_>) -> Result<()> {
        set(config, project).await?;
        config.persist_config_updates()?;
        Ok(())
    }

    pub async fn set_projects(config: &OckamConfig, projects: &[Project<'_>]) -> Result<()> {
        config.remove_projects_alias();
        for project in projects.iter() {
            set(config, project).await?;
        }
        config.persist_config_updates()?;
        Ok(())
    }

    pub fn remove_project(config: &OckamConfig, name: &str) -> Result<()> {
        config.remove_project_alias(name);
        config.persist_config_updates()?;
        Ok(())
    }

    pub fn get_project(config: &OckamConfig, name: &str) -> Option<String> {
        let inner = config.write();
        inner.lookup.get_project(name).map(|s| s.id.clone())
    }

    pub async fn refresh_projects(
        ctx: &Context,
        opts: &CommandGlobalOpts,
        api_node: &str,
        controller_route: &MultiAddr,
        tcp: Option<&TcpTransport>,
    ) -> Result<()> {
        let mut rpc = RpcBuilder::new(ctx, opts, api_node).tcp(tcp)?.build();
        rpc.request(api::project::list(controller_route)).await?;
        let projects = rpc.parse_response::<Vec<Project>>()?;
        set_projects(&opts.config, &projects).await?;
        Ok(())
    }
}
