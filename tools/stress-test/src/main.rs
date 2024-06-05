use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use clap::{Args, Parser, Subcommand};

use ockam::abac::tokio::runtime::Runtime;
use ockam::compat::tokio;
use ockam::tcp::{TcpListenerOptions, TcpTransport};
use ockam::{Context, NodeBuilder};
use ockam_api::nodes::service::{NodeManagerGeneralOptions, NodeManagerTransportOptions};
use ockam_api::nodes::{InMemoryNode, NodeManagerWorker, NODEMANAGER_ADDR};
use ockam_api::CliState;

use crate::config::Config;
use crate::portal_simulator::PortalStats;

mod config;
mod display;
mod execution;
mod portal_simulator;
mod stats;

#[derive(Debug, Args, Clone)]
struct RunCommand {
    config: PathBuf,
    #[arg(long)]
    log: bool,
}

#[derive(Debug, Args, Clone)]
struct ValidateCommand {
    config: PathBuf,
}

#[derive(Subcommand, Debug, Clone)]
enum Action {
    /// Run the stress test
    Run(RunCommand),
    /// Validate the configuration file
    Validate(ValidateCommand),
    /// Generate sample configuration files
    Generate,
}

#[derive(Debug, Parser, Clone)]
#[command(name = "stress-test")]
struct Main {
    /// Action to perform
    #[command(subcommand)]
    action: Action,
}

fn main() {
    let main: Main = Main::parse();
    match main.action {
        Action::Run(cmd) => run(cmd),
        Action::Validate(cmd) => validate(cmd),
        Action::Generate => generate(),
    }
}

fn generate() {
    println!("{}", Config::sample_configs());
}

fn validate(cmd: ValidateCommand) {
    match Config::parse(&cmd.config) {
        Ok(_config) => {
            println!("configuration file is valid");
        }
        Err(err) => {
            eprintln!("configuration file is invalid: {:?}", err.message());
            std::process::exit(1);
        }
    }
}

fn run(cmd: RunCommand) {
    let config = match Config::parse(&cmd.config) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("configuration file is invalid: {:?}", err.message());
            std::process::exit(1);
        }
    };

    let state = Arc::new(State::new(config, cmd.log));
    let runtime = Runtime::new().unwrap();

    {
        let state = state.clone();
        runtime.spawn(async move {
            loop {
                state.create_resources_for_delta_time().await;
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    state.display_loop(runtime);
}

struct Relay {
    failures_detected: u32,
    usages: u32,
}

const NODE_NAME: &str = "stress-tester";

struct State {
    relay_creation_failed: AtomicU32,
    portal_creation_failed: AtomicU32,
    portals: Arc<Mutex<HashMap<String, PortalStats>>>,
    relays: Arc<Mutex<HashMap<String, Relay>>>,
    begin: Instant,
    config: Config,
    node: Arc<InMemoryNode>,
    context: Arc<Context>,
    speed_stats: Arc<Mutex<Vec<(u64, u64)>>>,
    previous_bytes_sent: AtomicU64,
    previous_bytes_received: AtomicU64,
}

impl State {
    fn new(config: Config, log: bool) -> Self {
        let cli_state = CliState::with_default_dir().expect("cannot create cli state");
        let rt = Arc::new(Runtime::new().expect("cannot create a tokio runtime"));
        let builder = if log {
            NodeBuilder::new()
        } else {
            NodeBuilder::new().no_logging()
        };
        let (context, mut executor) = builder.with_runtime(rt.clone()).build();
        let context = Arc::new(context);

        // start the router, it is needed for the node manager creation
        rt.spawn(async move {
            executor
                .start_router()
                .await
                .expect("cannot start executor")
        });

        let runtime = context.runtime().clone();
        let node_manager = runtime
            .block_on(Self::make_node_manager(context.clone(), &cli_state))
            .expect("cannot create node manager");

        Self {
            relay_creation_failed: Default::default(),
            portal_creation_failed: Default::default(),
            portals: Default::default(),
            relays: Default::default(),
            speed_stats: Default::default(),
            previous_bytes_received: Default::default(),
            previous_bytes_sent: Default::default(),
            begin: Instant::now(),
            config,
            node: node_manager,
            context,
        }
    }

    async fn make_node_manager(
        ctx: Arc<Context>,
        cli_state: &CliState,
    ) -> ockam::Result<Arc<InMemoryNode>> {
        let tcp = TcpTransport::create(&ctx).await?;
        let options = TcpListenerOptions::new();
        let listener = tcp.listen(&"127.0.0.1:0", options).await?;

        let _ = cli_state
            .start_node_with_optional_values(NODE_NAME, &None, &None, Some(&listener))
            .await?;

        let trust_options = cli_state
            .retrieve_trust_options(&None, &None, &None, &None)
            .await?;

        let node_manager = Arc::new(
            InMemoryNode::new(
                &ctx,
                NodeManagerGeneralOptions::new(
                    cli_state.clone(),
                    NODE_NAME.to_string(),
                    true,
                    None,
                    false,
                ),
                NodeManagerTransportOptions::new(listener.flow_control_id().clone(), tcp),
                trust_options,
            )
            .await?,
        );

        let node_manager_worker = NodeManagerWorker::new(node_manager.clone());
        ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
            .await?;

        ctx.flow_controls()
            .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
        Ok(node_manager)
    }
}
