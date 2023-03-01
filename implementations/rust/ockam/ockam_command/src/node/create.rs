use clap::Args;
use minicbor::Decoder;
use ockam::identity::credential::Credential;
use ockam_identity::PublicIdentity;
use ockam_multiaddr::MultiAddr;
use ockam_vault::Vault;
use rand::prelude::random;
use tokio::io::AsyncBufReadExt;
use tokio::time::{sleep, Duration};

use anyhow::{anyhow, Context as _};
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};
use tracing::error;

use crate::project::ProjectInfo;
use crate::secure_channel::listener::create as secure_channel_listener;
use crate::service::config::Config;
use crate::service::start;
use crate::util::node_rpc;
use crate::util::{bind_to_port_check, embedded_node_that_is_not_stopped, exitcode};
use crate::{
    help, node::show::print_query_status, node::HELP_DETAIL, project, util::find_available_port,
    CommandGlobalOpts, Result,
};
use crate::{node::util::spawn_node, util::parse_node_name};
use crate::{
    node::util::{add_project_authority_from_project_info, init_node_state},
    util::RpcBuilder,
};
use ockam::{Address, AsyncTryClone};
use ockam::{Context, TcpTransport};
use ockam_api::{
    bootstrapped_identities_store::PreTrustedIdentities,
    config::cli::Authority,
    nodes::models::transport::{TransportMode, TransportType},
    nodes::{
        service::{
            NodeManagerGeneralOptions, NodeManagerProjectsOptions, NodeManagerTransportOptions,
        },
        NodeManager, NodeManagerWorker, NODEMANAGER_ADDR,
    },
};
use ockam_api::{config::cli, nodes::models::transport::CreateTransportJson};

use ockam_core::{AllowAll, LOCAL};

/// Create a node
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    /// Name of the node (Optional).
    #[arg(hide_default_value = true, default_value_t = hex::encode(&random::<[u8;4]>()))]
    pub node_name: String,

    /// Run the node in foreground.
    #[arg(display_order = 900, long, short)]
    pub foreground: bool,

    /// Watch stdin for EOF
    #[arg(display_order = 900, long = "exit-on-eof", short)]
    pub exit_on_eof: bool,

    /// TCP listener address
    #[arg(
        display_order = 900,
        long,
        short,
        id = "SOCKET_ADDRESS",
        default_value = "127.0.0.1:0"
    )]
    pub tcp_listener_address: String,

    /// ockam_command started a child process to run this node in foreground.
    #[arg(display_order = 900, long, hide = true)]
    pub child_process: bool,

    /// JSON config to setup a foreground node
    ///
    /// This argument is currently ignored on background nodes.  Node
    /// configuration is run asynchronously and may take several
    /// seconds to complete.
    #[arg(long, hide = true, value_parser=parse_launch_config)]
    pub launch_config: Option<Config>,

    #[arg(long, group = "trusted")]
    pub trusted_identities: Option<String>,
    #[arg(long, group = "trusted")]
    pub trusted_identities_file: Option<PathBuf>,
    #[arg(long, group = "trusted")]
    pub reload_from_trusted_identities_file: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub project: Option<PathBuf>,

    #[arg(long = "vault", value_name = "VAULT")]
    vault: Option<String>,

    #[arg(long = "identity", value_name = "IDENTITY")]
    identity: Option<String>,

    #[arg(long = "authority-identity", value_parser = parse_identity_authority)]
    pub authority_identities: Option<Vec<Authority>>,

    #[arg(long = "credential", value_name = "CREDENTIAL_NAME")]
    pub credential: Option<String>,
}

impl Default for CreateCommand {
    fn default() -> Self {
        Self {
            node_name: hex::encode(random::<[u8; 4]>()),
            exit_on_eof: false,
            tcp_listener_address: "127.0.0.1:0".to_string(),
            foreground: false,
            child_process: false,
            launch_config: None,
            project: None,
            vault: None,
            identity: None,
            trusted_identities: None,
            trusted_identities_file: None,
            reload_from_trusted_identities_file: None,
            authority_identities: None,
            credential: None,
        }
    }
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        if self.foreground {
            // Create a new node in the foreground (i.e. in this OS process)
            if let Err(e) = create_foreground_node(&options, &self) {
                error!(%e);
                eprintln!("{e:?}");
                std::process::exit(e.code());
            }
        } else {
            // Create a new node running in the background (i.e. another, new OS process)
            node_rpc(run_impl, (options, self))
        }
    }

    fn overwrite_addr(&self) -> Result<Self> {
        let cmd = self.clone();
        let addr: SocketAddr = if &cmd.tcp_listener_address == "127.0.0.1:0" {
            let port = find_available_port().context("failed to acquire available port")?;
            SocketAddr::new(IpAddr::from_str("127.0.0.1")?, port)
        } else {
            cmd.tcp_listener_address.parse()?
        };
        Ok(Self {
            tcp_listener_address: addr.to_string(),
            ..cmd
        })
    }
}

fn parse_launch_config(config_or_path: &str) -> Result<Config> {
    match serde_json::from_str::<Config>(config_or_path) {
        Ok(c) => Ok(c),
        Err(_) => {
            let path = PathBuf::from_str(config_or_path).context(anyhow!("Not a valid path"))?;
            Config::read(path)
        }
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> crate::Result<()> {
    let node_name = &parse_node_name(&cmd.node_name)?;

    if cmd.child_process {
        return Err(crate::Error::new(
            exitcode::CONFIG,
            anyhow!("Cannot create a background node from background node"),
        ));
    }

    // Spawn node in another, new process
    let cmd = cmd.overwrite_addr()?;
    let addr = SocketAddr::from_str(&cmd.tcp_listener_address)?;

    spawn_background_node(&ctx, &opts, &cmd, addr).await?;

    // Print node status
    let tcp = TcpTransport::create(&ctx).await?;
    let mut rpc = RpcBuilder::new(&ctx, &opts, node_name).tcp(&tcp)?.build();
    let mut is_default = false;
    if let Ok(state) = opts.state.nodes.default() {
        is_default = &state.config.name == node_name;
    }
    print_query_status(&mut rpc, node_name, true, is_default).await?;

    Ok(())
}

fn create_foreground_node(opts: &CommandGlobalOpts, cmd: &CreateCommand) -> crate::Result<()> {
    let cmd = cmd.overwrite_addr()?;
    let addr = SocketAddr::from_str(&cmd.tcp_listener_address)?;
    embedded_node_that_is_not_stopped(run_foreground_node, (opts.clone(), cmd, addr))?;
    Ok(())
}

async fn run_foreground_node(
    mut ctx: Context,
    (opts, cmd, addr): (CommandGlobalOpts, CreateCommand, SocketAddr),
) -> crate::Result<()> {
    let cfg = &opts.config;
    let node_name = parse_node_name(&cmd.node_name)?;

    // This node was initially created as a foreground node
    // and there is no existing state for it yet.
    if !cmd.child_process && opts.state.nodes.get(&node_name).is_err() {
        init_node_state(
            &ctx,
            &opts,
            &node_name,
            cmd.vault.as_ref(),
            cmd.identity.as_ref(),
        )
        .await?;
    }

    if let Some(authority_identities) = cmd.authority_identities {
        for auth in authority_identities.into_iter() {
            let vault = Vault::default();
            let i = PublicIdentity::import(auth.identity(), &vault).await?;
            cfg.authorities(&node_name)?
                .add_authority(i.identifier().clone(), auth)?;
        }
    }

    let project_id = match &cmd.project {
        Some(path) => {
            let s = tokio::fs::read_to_string(path).await?;
            let p: ProjectInfo = serde_json::from_str(&s)?;
            let project_id = p.id.to_string();
            project::config::set_project(cfg, &(&p).into()).await?;
            add_project_authority_from_project_info(p, &node_name, cfg).await?;
            Some(project_id)
        }
        None => None,
    };

    let tcp = TcpTransport::create(&ctx).await?;
    let bind = cmd.tcp_listener_address;
    let (socket_addr, listener_addr) = tcp.listen(&bind).await?;

    let node_state = opts.state.nodes.get(&node_name)?;
    let setup_config = node_state.setup()?;
    node_state.set_setup(
        &setup_config
            .set_verbose(opts.global_args.verbose)
            .add_transport(CreateTransportJson::new(
                TransportType::Tcp,
                TransportMode::Listen,
                &bind,
            )?),
    )?;

    let pre_trusted_identities = match (
        cmd.trusted_identities,
        cmd.trusted_identities_file,
        cmd.reload_from_trusted_identities_file,
    ) {
        (Some(val), _, _) => Some(PreTrustedIdentities::new_from_string(&val)?),
        (_, Some(val), _) => Some(PreTrustedIdentities::new_from_disk(val, false)?),
        (_, _, Some(val)) => Some(PreTrustedIdentities::new_from_disk(val, true)?),
        _ => None,
    };
    let projects = cfg.inner().lookup().projects().collect();

    let credential = match cmd.credential {
        Some(cred_name) => Some(
            opts.state
                .credentials
                .get(&cred_name)?
                .config()
                .await?
                .credential()?,
        ),
        None => None,
    };

    let node_man = NodeManager::create(
        &ctx,
        NodeManagerGeneralOptions::new(
            opts.state.clone(),
            cmd.node_name.clone(),
            cmd.launch_config.is_some(),
            pre_trusted_identities,
        ),
        NodeManagerProjectsOptions::new(
            Some(&cfg.authorities(&node_name)?.snapshot()),
            project_id,
            projects,
            credential,
        ),
        NodeManagerTransportOptions::new(
            (
                TransportType::Tcp,
                TransportMode::Listen,
                listener_addr,
                socket_addr.to_string(),
            ),
            tcp.async_try_clone().await?,
        ),
    )
    .await?;
    let node_manager_worker = NodeManagerWorker::new(node_man);

    ctx.start_worker(
        NODEMANAGER_ADDR,
        node_manager_worker,
        AllowAll, // FIXME: @ac
        AllowAll, // FIXME: @ac
    )
    .await?;

    if let Some(path) = &cmd.launch_config {
        let node_opts = super::NodeOpts {
            api_node: node_name.clone(),
        };
        if start_services(&ctx, &tcp, path, addr, node_opts, &opts)
            .await
            .is_err()
        {
            //TODO: Process should terminate on any error during its setup phase,
            //      not just during the start_services.
            //TODO: This sleep here is a workaround on some orchestrated environment,
            //      the lmdb db, that is used for policy storage, failes to be re-opened
            //      if it's still opened from another docker container, where they share
            //      the same pid. By sleeping for a while we let this container be promoted
            //      and the other being terminated, so when restarted it works.  This is
            //      FAR from ideal.
            sleep(Duration::from_secs(10)).await;
            std::process::exit(exitcode::SOFTWARE);
        }
    }

    if cmd.exit_on_eof {
        stop_node_on_eof(&mut ctx, &opts, &node_name).await?;
    }

    Ok(())
}

// Read STDIN until EOF is encountered and then stop the node
async fn stop_node_on_eof(
    ctx: &mut Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
) -> crate::Result<()> {
    let reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = reader.lines();

    loop {
        match lines.next_line().await {
            Ok(Some(_)) => (),
            Ok(None) => break,
            Err(_) => unreachable!(),
        }
    }

    ctx.stop().await?;
    opts.state.nodes.get(node_name)?.kill_process(false)?;
    Ok(())
}

async fn start_services(
    ctx: &Context,
    tcp: &TcpTransport,
    cfg: &Config,
    addr: SocketAddr,
    node_opts: super::NodeOpts,
    opts: &CommandGlobalOpts,
) -> Result<()> {
    let config = {
        if let Some(sc) = &cfg.startup_services {
            sc.clone()
        } else {
            return Ok(());
        }
    };

    // Checking if node accepts connections
    let addr = tcp.connect_socket(addr).await?;

    if let Some(cfg) = config.vault {
        if !cfg.disabled {
            println!("starting vault service ...");
            start::start_vault_service(ctx, opts, &node_opts.api_node, &cfg.address, Some(tcp))
                .await?
        }
    }
    if let Some(cfg) = config.identity {
        if !cfg.disabled {
            println!("starting identity service ...");
            start::start_identity_service(ctx, opts, &node_opts.api_node, &cfg.address, Some(tcp))
                .await?
        }
    }
    if let Some(cfg) = config.secure_channel_listener {
        if !cfg.disabled {
            let adr = Address::from((LOCAL, cfg.address));
            let ids = cfg.authorized_identifiers;
            let identity = cfg.identity;
            let rte = addr.clone().into();
            println!("starting secure-channel listener ...");
            secure_channel_listener::create_listener(ctx, adr, ids, identity, rte).await?;
        }
    }
    if let Some(cfg) = config.verifier {
        if !cfg.disabled {
            println!("starting verifier service ...");
            start::start_verifier_service(ctx, opts, &node_opts.api_node, &cfg.address, Some(tcp))
                .await?
        }
    }
    if let Some(cfg) = config.authenticator {
        if !cfg.disabled {
            println!("starting authenticator service ...");
            start::start_authenticator_service(
                ctx,
                opts,
                &node_opts.api_node,
                &cfg.address,
                &cfg.project,
                Some(tcp),
            )
            .await?
        }
    }
    if let Some(cfg) = config.okta_identity_provider {
        if !cfg.disabled {
            println!("starting okta identity provider service ...");
            start::start_okta_identity_provider(ctx, opts, &node_opts.api_node, &cfg, Some(tcp))
                .await?
        }
    }

    Ok(())
}

async fn spawn_background_node(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    cmd: &CreateCommand,
    addr: SocketAddr,
) -> crate::Result<()> {
    // Check if the port is used by some other services or process
    if !bind_to_port_check(&addr) {
        return Err(crate::Error::new(
            exitcode::IOERR,
            anyhow!("Another process is listening on the provided port!"),
        ));
    }

    let node_name = parse_node_name(&cmd.node_name)?;

    // Create node state, including the vault and identity if don't exist
    init_node_state(
        ctx,
        opts,
        &node_name,
        cmd.vault.as_ref(),
        cmd.identity.as_ref(),
    )
    .await?;

    // Construct the arguments list and re-execute the ockam
    // CLI in foreground mode to start the newly created node
    spawn_node(
        opts,
        opts.global_args.verbose,
        &node_name,
        &cmd.tcp_listener_address,
        cmd.project.as_deref(),
        cmd.trusted_identities.as_ref(),
        cmd.trusted_identities_file.as_ref(),
        cmd.reload_from_trusted_identities_file.as_ref(),
        cmd.launch_config
            .as_ref()
            .map(|config| serde_json::to_string(config).unwrap()),
        cmd.authority_identities.as_ref(),
        cmd.credential.as_ref(),
    )?;

    Ok(())
}

fn otc_parser(val: &str) -> Result<OneTimeCode> {
    let bytes = hex::decode(val)?;
    let code = <[u8; 32]>::try_from(bytes.as_slice()).context("Failed to parse OTC")?;
    Ok(code.into())
}

fn parse_identity_authority(identity: &str) -> Result<Authority> {
    let identity_as_bytes = match hex::decode(identity) {
        Ok(b) => b,
        Err(e) => return Err(anyhow!(e).into()),
    };

    // TODO: FIXME - Identity Authorities do not have an address `/secure` is being used as a placeholder
    //        -  Oakley
    let a = cli::Authority::new(identity_as_bytes, MultiAddr::from_str("/secure")?);
    Ok(a)
}
