use std::sync::Arc;

use miette::IntoDiagnostic;
use ockam_multiaddr::MultiAddr;
use tauri::async_runtime::{block_on, spawn, RwLock};
use tracing::{error, info};

use ockam::Context;
use ockam::{NodeBuilder, TcpListenerOptions, TcpTransport};
use ockam_api::cli_state::{CliState, StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};
use ockam_api::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerTransportOptions, NodeManagerTrustOptions,
};
use ockam_api::nodes::{NodeManager, NodeManagerWorker, NODEMANAGER_ADDR};
use ockam_command::node::util::{add_project_info_to_node_state, init_node_state};
use ockam_command::util::api::{CloudOpts, TrustContextConfigBuilder, TrustContextOpts};
use ockam_command::{CommandGlobalOpts, GlobalArgs, Terminal};

use crate::app::model_state::ModelState;
use crate::app::model_state_repository::{LmdbModelStateRepository, ModelStateRepository};
use crate::Result;

pub const NODE_NAME: &str = "ockam_app";
// TODO: static project name of "default" is an unsafe default behavior due to backend uniqueness requirements
pub const PROJECT_NAME: &str = "default";

/// The AppState struct contains all the state managed by `tauri`.
/// It can be retrieved with the `AppHandle<Wry>` parameter and the `AppHandle::state()` method
/// Note that it contains a `NodeManagerWorker`. This makes the desktop app a full-fledged node
/// with its own set of secure channels, outlets, transports etc...
/// However there is no associated persistence yet so outlets created with this `NodeManager` will
/// have to be recreated when the application restarts.
pub struct AppState {
    context: Arc<Context>,
    global_args: GlobalArgs,
    state: Arc<RwLock<CliState>>,
    controller_address: Arc<MultiAddr>,
    node_manager_worker: Arc<RwLock<NodeManagerWorker>>,
    model_state: Arc<RwLock<ModelState>>,
    model_state_repository: Arc<RwLock<Arc<dyn ModelStateRepository>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create a new AppState
    pub fn new() -> AppState {
        let options = CommandGlobalOpts::new(GlobalArgs::default().set_quiet());
        let (context, mut executor) = NodeBuilder::new().no_logging().build();
        let context = Arc::new(context);

        // from now on we use the same runtime everywhere we need to run an async action
        tauri::async_runtime::set(context.runtime().clone());
        // start the router, it is needed for the node manager creation
        spawn(async move { executor.start_router().await });
        let node_manager_worker = create_node_manager_worker(context.clone(), options.clone());
        let model_state_repository = create_model_state_repository(options.clone().state);
        let model_state = load_model_state(
            model_state_repository.clone(),
            &node_manager_worker,
            context.clone(),
            &options.state,
        );

        AppState {
            context,
            global_args: options.global_args,
            state: Arc::new(RwLock::new(options.state)),
            controller_address: Arc::new(CloudOpts::route()),
            node_manager_worker: Arc::new(RwLock::new(node_manager_worker)),
            model_state: Arc::new(RwLock::new(model_state)),
            model_state_repository: Arc::new(RwLock::new(model_state_repository)),
        }
    }

    pub async fn reset(&self) -> miette::Result<()> {
        self.reset_state().await?;
        self.reset_node_manager().await?;

        // recreate the model state repository since the cli state has changed
        let identity_path = self
            .state()
            .await
            .identities
            .identities_repository_path()
            .unwrap();
        let new_state_repository = LmdbModelStateRepository::new(identity_path).await?;
        let mut model_state_repository = self.model_state_repository.write().await;
        *model_state_repository = Arc::new(new_state_repository);

        Ok(())
    }

    async fn reset_state(&self) -> miette::Result<()> {
        let mut state = self.state.write().await;
        match state.reset().await {
            Ok(s) => {
                *state = s;
                info!("reset the cli state");
            }
            Err(e) => error!("Failed to reset the state {e:?}"),
        }
        Ok(())
    }

    /// Recreate a new NodeManagerWorker instance, which will restart the necessary
    /// child workers as described in its Worker trait implementation.
    pub async fn reset_node_manager(&self) -> miette::Result<()> {
        let mut node_manager_worker = self.node_manager_worker.write().await;
        let _ = node_manager_worker.stop(&self.context).await;
        info!("stopped the old node manager");

        for w in self.context.list_workers().await.into_diagnostic()? {
            let _ = self.context.stop_worker(w.address()).await;
        }
        info!("stopped all the ctx workers");

        let new_node_manager =
            make_node_manager_worker(self.context.clone(), self.options().await).await?;
        *node_manager_worker = new_node_manager;
        info!("set a new node manager");
        Ok(())
    }

    /// Return the application Context
    /// This can be used to run async actions involving the Router
    pub fn context(&self) -> Arc<Context> {
        self.context.clone()
    }

    /// Returns the address being used to contact Orchestrator
    pub fn controller_address(&self) -> Arc<MultiAddr> {
        self.controller_address.clone()
    }

    /// Return the application cli state
    /// This can be used to manage the on-disk state for projects, identities, vaults, etc...
    pub async fn state(&self) -> CliState {
        let state = self.state.read().await;
        state.clone()
    }

    /// Return the node manager worker
    pub async fn node_manager_worker(&self) -> NodeManagerWorker {
        let node_manager = self.node_manager_worker.read().await;
        node_manager.clone()
    }

    /// Return the global options with a quiet terminal
    pub async fn options(&self) -> CommandGlobalOpts {
        CommandGlobalOpts {
            global_args: self.global_args.clone(),
            state: self.state().await,
            terminal: Terminal::quiet(),
        }
    }

    /// Return true if the user is enrolled
    /// At the moment this check only verifies that there is a default project.
    /// This project should be the project that is created at the end of the enrollment procedure
    pub async fn is_enrolled(&self) -> Result<bool> {
        let identity_state = self.state().await.identities.get_or_default(None)?;

        if !identity_state.is_enrolled() {
            return Ok(false);
        }

        let default_space_exists = self.state().await.spaces.default().is_ok();
        if !default_space_exists {
            return Err(
                "There should be a default space set for the current user. Please re-enroll".into(),
            );
        }

        let default_project_exists = self.state().await.projects.default().is_ok();
        if !default_project_exists {
            return Err(
                "There should be a default project set for the current user. Please re-enroll"
                    .into(),
            );
        }

        Ok(true)
    }

    /// Return the list of currently running outlets
    pub async fn tcp_outlet_list(&self) -> Vec<OutletStatus> {
        let node_manager = self.node_manager_worker.read().await;
        node_manager.list_outlets().await.list
    }

    pub async fn user_email(&self) -> Option<String> {
        self.model(|m| m.get_user_info()).await.map(|ui| ui.email)
    }

    pub async fn model_mut(&self, f: impl FnOnce(&mut ModelState)) -> Result<()> {
        let mut model_state = self.model_state.write().await;
        f(&mut model_state);
        self.model_state_repository
            .read()
            .await
            .store(&model_state)
            .await?;
        Ok(())
    }

    pub async fn model<T>(&self, f: impl FnOnce(&ModelState) -> T) -> T {
        let mut model_state = self.model_state.read().await;
        f(&mut model_state)
    }
}

/// Create a node manager worker
fn create_node_manager_worker(ctx: Arc<Context>, opts: CommandGlobalOpts) -> NodeManagerWorker {
    let options = opts;
    match block_on(async { make_node_manager_worker(ctx.clone(), options).await }) {
        Ok(w) => w,
        Err(e) => {
            println!("cannot create a node manager: {e:?}");
            panic!("{e}")
        }
    }
}

/// Make a node manager with a default node called "default"
pub(crate) async fn make_node_manager_worker(
    ctx: Arc<Context>,
    opts: CommandGlobalOpts,
) -> miette::Result<NodeManagerWorker> {
    init_node_state(&opts, NODE_NAME, None, None).await?;

    let tcp = TcpTransport::create(&ctx).await.into_diagnostic()?;
    let options = TcpListenerOptions::new();
    let listener = tcp
        .listen(&"127.0.0.1:0", options)
        .await
        .into_diagnostic()?;

    let trust_context_opts = default_trust_context_opts(&opts.state);
    add_project_info_to_node_state(NODE_NAME, &opts, &trust_context_opts).await?;
    let trust_context_config =
        TrustContextConfigBuilder::new(&opts.state, &trust_context_opts)?.build();

    let node_state = opts.state.nodes.get(NODE_NAME)?;
    node_state.set_setup(
        &node_state.config().setup_mut().set_api_transport(
            CreateTransportJson::new(
                TransportType::Tcp,
                TransportMode::Listen,
                &listener.socket_address().to_string(),
            )
            .into_diagnostic()?,
        ),
    )?;

    let node_manager = NodeManager::create(
        &ctx,
        NodeManagerGeneralOptions::new(opts.state.clone(), NODE_NAME.to_string(), false, None),
        NodeManagerTransportOptions::new(listener.flow_control_id().clone(), tcp),
        NodeManagerTrustOptions::new(trust_context_config),
    )
    .await
    .into_diagnostic()?;

    let node_manager_worker = NodeManagerWorker::new(node_manager);
    ctx.flow_controls()
        .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
    let _ = ctx
        .start_worker(NODEMANAGER_ADDR, node_manager_worker.clone())
        .await
        .into_diagnostic();

    Ok(node_manager_worker)
}

pub(crate) fn default_trust_context_opts(state: &CliState) -> TrustContextOpts {
    TrustContextOpts {
        project: state.projects.default().ok().map(|p| p.name().to_string()),
        ..Default::default()
    }
}

/// Create the repository containing the model state
fn create_model_state_repository(state: CliState) -> Arc<dyn ModelStateRepository> {
    let identity_path = state.identities.identities_repository_path().unwrap();
    match block_on(async move { LmdbModelStateRepository::new(identity_path).await }) {
        Ok(model_state_repository) => Arc::new(model_state_repository),
        Err(e) => {
            println!("cannot create a model state repository manager: {e:?}");
            panic!("{}", e)
        }
    }
}

/// Load a previously persisted ModelState
fn load_model_state(
    model_state_repository: Arc<dyn ModelStateRepository>,
    node_manager_worker: &NodeManagerWorker,
    context: Arc<Context>,
    cli_state: &CliState,
) -> ModelState {
    block_on(async {
        match model_state_repository.load().await {
            Ok(model_state) => {
                let model_state = model_state.unwrap_or(ModelState::default());
                crate::shared_service::tcp_outlet::load_model_state(
                    context.clone(),
                    node_manager_worker,
                    &model_state,
                )
                .await;
                crate::shared_service::relay::load_model_state(
                    context.clone(),
                    node_manager_worker,
                    cli_state,
                )
                .await;
                model_state
            }
            Err(e) => {
                println!("cannot load the model state: {e:?}");
                panic!("{}", e)
            }
        }
    })
}
