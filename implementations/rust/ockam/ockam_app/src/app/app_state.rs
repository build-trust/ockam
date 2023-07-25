use std::sync::{Arc, RwLock};

use miette::IntoDiagnostic;

use ockam::Context;
use ockam::{NodeBuilder, TcpListenerOptions, TcpTransport};
use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_api::cloud::enroll::auth0::UserInfo;
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
    pub(crate) node_manager: NodeManagerWorker,
    model_state: Arc<RwLock<ModelState>>,
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

        AppState {
            context,
            global_args: options.global_args,
            state: options.state,
            node_manager: NodeManagerWorker::new(node_manager),
            model_state: Arc::new(RwLock::new(ModelState::default())),
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

    /// Return the global options with a quiet terminal
    pub fn options(&self) -> CommandGlobalOpts {
        CommandGlobalOpts {
            global_args: self.global_args.clone(),
            state: self.state.clone(),
            terminal: Terminal::quiet(),
        }
    }

    pub fn is_enrolled(&self) -> bool {
        self.state.projects.default().is_ok()
    }

    pub async fn tcp_outlet_list(&self) -> Vec<OutletStatus> {
        let node_manager = self.node_manager.get().read().await;
        node_manager.list_outlets().list
    }

    pub async fn set_user_info(&self, user_info: UserInfo) {
        let mut model_state = self.model_state.write().unwrap();
        model_state.set_user_info(user_info);
    }

    pub async fn get_user_info(&self) -> Option<UserInfo> {
        let model_state = self.model_state.read().unwrap();
        model_state.get_user_info()
    }
}

/// Create a node manager
fn create_node_manager(ctx: Arc<Context>, opts: CommandGlobalOpts) -> NodeManager {
    let options = opts;
    match tauri::async_runtime::block_on(async { make_node_manager(ctx.clone(), options).await }) {
        Ok(node_manager) => node_manager,
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
