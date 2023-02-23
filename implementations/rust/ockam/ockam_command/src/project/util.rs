use std::io::Write;
use std::str::FromStr;

use anyhow::{anyhow, Context as _};
use ockam_core::api::Request;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;
use tracing::debug;

use ockam::identity::IdentityIdentifier;
use ockam::TcpTransport;
use ockam_api::authenticator::direct::types::AddMember;
use ockam_api::cloud::project::Project;
use ockam_api::config::lookup::{LookupMeta, ProjectAuthority, ProjectLookup};
use ockam_api::multiaddr_to_addr;
use ockam_api::nodes::models::{self, secure_channel::*};
use ockam_multiaddr::{MultiAddr, Protocol};

use crate::project::enroll::replace_project;
use crate::util::api::CloudOpts;
use crate::util::{api, RpcBuilder};
use crate::{CommandGlobalOpts, OckamConfig, Result};

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
                .context(format!("Failed to get project {name} from config lookup"))?;
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
                None,
            )
            .await?,
        );
    }

    // There should be the same number of project occurrences in the
    // input MultiAddr than there are in the secure channels vector.
    assert_eq!(meta.project.len(), sc.len());
    Ok(sc)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_secure_channel_to_project(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    api_node: &str,
    tcp: Option<&TcpTransport>,
    project_access_route: &MultiAddr,
    project_identity: &str,
    credential_exchange_mode: CredentialExchangeMode,
    identity: Option<String>,
) -> crate::Result<MultiAddr> {
    let authorized_identifier = vec![IdentityIdentifier::from_str(project_identity)?];
    let mut rpc = RpcBuilder::new(ctx, opts, api_node).tcp(tcp)?.build();

    let payload = models::secure_channel::CreateSecureChannelRequest::new(
        project_access_route,
        Some(authorized_identifier),
        credential_exchange_mode,
        identity,
    );
    let req = Request::post("/node/secure_channel").body(payload);
    rpc.request(req).await?;

    let sc = rpc.parse_response::<CreateSecureChannelResponse>()?;
    Ok(sc.addr()?)
}

pub async fn create_secure_channel_to_authority(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    authority: &ProjectAuthority,
    addr: &MultiAddr,
    identity: Option<String>,
) -> crate::Result<MultiAddr> {
    let mut rpc = RpcBuilder::new(ctx, opts, node_name).build();
    debug!(%addr, "establishing secure channel to project authority");
    let allowed = vec![authority.identity_id().clone()];
    let payload = models::secure_channel::CreateSecureChannelRequest::new(
        addr,
        Some(allowed),
        CredentialExchangeMode::None,
        identity,
    );
    let req = Request::post("/node/secure_channel").body(payload);
    rpc.request(req).await?;
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
    // 3 Mins (180 Seconds) per strategy with 5 second intervals
    let retry_strategy = FixedInterval::from_millis(5000).take(36);

    // Persist project config prior to checking readiness which might take a while
    config::set_project_id(&opts.config, &project).await?;

    // Check if Project and Project Authority info is available
    println!("Project created!");
    if !project.is_ready() {
        print!("Waiting for it to be ready...");
        let cloud_route = &cloud_opts.route();
        let project_id = project.id.clone();
        project = Retry::spawn(retry_strategy.clone(), || async {
            std::io::stdout().flush()?;
            let mut rpc = RpcBuilder::new(ctx, opts, api_node).build();

            // Handle the project show request result
            // so we can provide better errors in the case orchestrator does not respond timely
            if let Ok(_) = rpc
                .request(api::project::show(&project_id, cloud_route))
                .await
            {
                let p = rpc.parse_response::<Project>()?;
                if p.is_ready() {
                    println!("✅");
                    return Ok(p.to_owned());
                }
            }
            print!(".");
            Err(anyhow!("Project creation timed out. Plaese try again."))
        })
        .await?;

        println!();
    }

    {
        print!("Establishing connection (this can take a few minutes)...");
        Retry::spawn(retry_strategy.clone(), || async {
            std::io::stdout().flush()?;

            // Handle the reachable result, so we can provide better errors in the case a project isn't
            if let Ok(reachable) = project.is_reachable().await {
                if reachable {
                    println!("✅");
                    return Ok(());
                }
            }

            print!(".");
            Err(anyhow!("Timed out while trying to establish a connection to the project. Please try again."))
        }).await?;

        println!();
    }

    {
        print!("Establishing secure channel...");
        std::io::stdout().flush()?;

        let project_route = project.access_route()?;
        let project_identity = project
            .identity
            .as_ref()
            .context("Project identity is not set.")?
            .to_string();

        Retry::spawn(retry_strategy.clone(), || async {
            std::io::stdout().flush()?;
            if let Ok(sc_addr) = create_secure_channel_to_project(
                ctx,
                opts,
                api_node,
                tcp,
                &project_route,
                &project_identity,
                CredentialExchangeMode::None,
                None,
            )
            .await
            {
                // Try to delete secure channel, ignore result.
                let _ = delete_secure_channel(ctx, opts, api_node, tcp, &sc_addr).await;
                println!("✅");
                return Ok(());
            }
            print!(".");
            Err(anyhow!("Timed out while trying to establish a secure channel to the project. Please try again."))
        })
        .await?;

        println!();
    }

    {
        print!("Establishing secure channel to authority...");
        std::io::stdout().flush()?;

        let authority = ProjectAuthority::from_raw(
            &project.authority_access_route,
            &project.authority_identity,
        )
        .await?
        .context("Project does not have an authority defined.")?;

        Retry::spawn(retry_strategy.clone(), || async {
            std::io::stdout().flush()?;
            if let Ok(sc_addr) = create_secure_channel_to_authority(
                ctx,
                opts,
                api_node,
                &authority,
                authority.address(),
                None,
            )
            .await
            {
                // Try to delete secure channel, ignore result.
                let _ = delete_secure_channel(ctx, opts, api_node, tcp, &sc_addr).await;
                println!("✅");
                return Ok(());
            }

            print!(".");
            Err(anyhow!("Time out while trying to establish a secure channel to the project authority. Please try again."))
        })
        .await?;

        println!();
    }

    // Persist project config with all its fields
    config::set_project(&opts.config, &project).await?;
    Ok(project)
}

pub async fn project_enroll_admin(
    ctx: &ockam::Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    project: &Project<'_>,
) -> Result<()> {
    println!("Enrolling as a member of the project...");
    let node_state = opts.state.nodes.get(node_name)?;
    let identifier = node_state.config.identity_config()?.identifier;
    let authority =
        ProjectAuthority::from_raw(&project.authority_access_route, &project.authority_identity)
            .await?
            .ok_or_else(|| anyhow!("Authority details not configured"))?;
    let worker_addr =
        MultiAddr::from_str(&format!("/project/{}/service/authenticator", &project.name))?;
    let authenticator_address = replace_project(&worker_addr, authority.address())?;
    let to = {
        let mut addr = create_secure_channel_to_authority(
            ctx,
            opts,
            node_name,
            &authority,
            &authenticator_address,
            None,
        )
        .await?;
        for proto in worker_addr.iter().skip(1) {
            addr.push_back_value(&proto).map_err(anyhow::Error::from)?
        }
        addr
    };

    let req = Request::post("/members").body(AddMember::new(identifier));
    let mut rpc = RpcBuilder::new(ctx, opts, node_name).to(&to)?.build();
    rpc.request(req).await?;
    rpc.is_ok()
}

pub mod config {
    use crate::util::output::Output;
    use ockam::Context;

    use tracing::trace;

    use super::*;

    async fn set(config: &OckamConfig, project: &Project<'_>) -> Result<()> {
        if !project.is_ready() {
            trace!("Project is not ready yet {}", project.output()?);
            return Err(
                anyhow!("Project is not ready yet, wait a few seconds and try again").into(),
            );
        }

        config.set_project_alias(
            project.name.to_string(),
            ProjectLookup::from_project(project).await?,
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
