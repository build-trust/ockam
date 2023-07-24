use std::sync::{Arc, Mutex};

use miette::IntoDiagnostic;

use ockam::Context;
use ockam::{NodeBuilder, TcpListenerOptions, TcpTransport};
use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_api::config::lookup::ProjectLookup;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerProjectsOptions, NodeManagerTransportOptions,
    NodeManagerTrustOptions,
};
use ockam_api::nodes::{NodeManager, NodeManagerWorker};
use ockam_command::node::util::init_node_state;
use ockam_command::util::api::{TrustContextConfigBuilder, TrustContextOpts};
use ockam_command::{CommandGlobalOpts, GlobalArgs, Terminal};

use crate::app::model_state::ModelState;

pub const NODE_NAME: &str = "default";
pub const SPACE_NAME: &str = "default";
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
    state: CliState,
    node_manager: NodeManagerWorker,
    model_state: Mutex<ModelState>,
}

impl From<AppState> for CommandGlobalOpts {
    fn from(app_state: AppState) -> CommandGlobalOpts {
        app_state.options()
    }
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
        tauri::async_runtime::spawn(async move { executor.start_router().await });
        let node_manager = create_node_manager(context.clone(), options.clone());

        // initialize the model state
        let model_state = Self::initialize_model_state(options.clone(), node_manager.clone());

        AppState {
            context,
            global_args: options.global_args,
            state: options.state,
            node_manager,
            model_state: Mutex::new(model_state),
        }
    }

    /// Return the application Context
    /// This can be used to run async actions involving the Router
    pub fn context(&self) -> Arc<Context> {
        self.context.clone()
    }

    /// Return the application cli state
    /// This can be used to manage the on-disk state for projects, identities, vaults, etc...
    pub fn state(&self) -> CliState {
        self.state.clone()
    }

    /// Return the node manager associated to the application
    pub fn node_manager(&self) -> NodeManagerWorker {
        self.node_manager.clone()
    }

    /// Return the global options with a quiet terminal
    pub fn options(&self) -> CommandGlobalOpts {
        CommandGlobalOpts {
            global_args: self.global_args.clone(),
            state: self.state.clone(),
            terminal: Terminal::quiet(),
        }
    }

    pub fn is_enrolled(&self) -> bool {
        self.model_state.lock().unwrap().is_enrolled
    }

    pub fn tcp_outlet_list(&self) -> Vec<OutletStatus> {
        self.model_state.lock().unwrap().outlets.clone()
    }

    pub fn set_enrolled(&self) {
        let mut model_state = self.model_state.lock().unwrap();
        model_state.set_enrolled();
    }

    pub fn add_outlet(&self, outlet: OutletStatus) {
        let mut model_state = self.model_state.lock().unwrap();
        model_state.add_outlet(outlet);
    }

    pub fn reset(&self) {
        let mut model_state = self.model_state.lock().unwrap();
        *model_state = ModelState::default();
    }

    fn initialize_model_state(
        options: CommandGlobalOpts,
        node_manager: NodeManagerWorker,
    ) -> ModelState {
        let outlets =
            tauri::async_runtime::block_on(async { node_manager.list_outlets().await.list });
        let is_enrolled = options.state.projects.default().is_ok();
        ModelState::new(is_enrolled, outlets)
    }
}

/// Create a node manager
fn create_node_manager(ctx: Arc<Context>, opts: CommandGlobalOpts) -> NodeManagerWorker {
    let options = opts;
    match tauri::async_runtime::block_on(async { make_node_manager(ctx.clone(), options).await }) {
        Ok(node_manager) => NodeManagerWorker::new(node_manager),
        Err(e) => {
            println!("cannot create a node manager: {:?}", e);
            panic!("{}", e)
        }
    }
}

/// Make a node manager with a default node called "default"
async fn make_node_manager(
    ctx: Arc<Context>,
    opts: CommandGlobalOpts,
) -> miette::Result<NodeManager> {
    init_node_state(&opts, NODE_NAME, None, None).await?;

    let tcp = TcpTransport::create(&ctx).await.into_diagnostic()?;
    let options = TcpListenerOptions::new();
    let listener = tcp
        .listen(&"127.0.0.1:0", options)
        .await
        .into_diagnostic()?;
    let projects = ProjectLookup::from_state(opts.state.projects.list()?).await?;
    let trust_context_config =
        TrustContextConfigBuilder::new(&opts.state, &TrustContextOpts::default())?
            .with_authority_identity(None)
            .with_credential_name(None)
            .build();

    let node_manager = NodeManager::create(
        &ctx,
        NodeManagerGeneralOptions::new(opts.state.clone(), NODE_NAME.to_string(), false, None),
        NodeManagerProjectsOptions::new(projects),
        NodeManagerTransportOptions::new(listener.flow_control_id().clone(), tcp),
        NodeManagerTrustOptions::new(trust_context_config),
    )
    .await
    .into_diagnostic()?;
    Ok(node_manager)
}
