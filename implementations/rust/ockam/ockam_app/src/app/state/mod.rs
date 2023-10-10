use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use miette::IntoDiagnostic;
use tauri::async_runtime::{block_on, spawn, RwLock};
use tauri::{AppHandle, Manager, Runtime};
use tracing::{error, info, trace, warn};

pub(crate) use crate::app::state::model::ModelState;
pub(crate) use crate::app::state::repository::{LmdbModelStateRepository, ModelStateRepository};
use crate::background_node::{BackgroundNodeClient, Cli};
use ockam::Context;
use ockam::{NodeBuilder, TcpListenerOptions, TcpTransport};
use ockam_api::cli_state::{
    add_project_info_to_node_state, init_node_state, CliState, StateDirTrait, StateItemTrait,
};
use ockam_api::cloud::enroll::auth0::UserInfo;
use ockam_api::cloud::Controller;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::models::transport::{CreateTransportJson, TransportMode, TransportType};
use ockam_api::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerTransportOptions, NodeManagerTrustOptions,
};
use ockam_api::nodes::InMemoryNode;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_api::trust_context::TrustContextConfigBuilder;

use crate::Result;

mod model;
mod repository;

pub const NODE_NAME: &str = "ockam_app";
// TODO: static project name of "default" is an unsafe default behavior due to backend uniqueness requirements
pub const PROJECT_NAME: &str = "default";

/// The AppState struct contains all the state managed by `tauri`.
/// It can be retrieved with the `AppHandle<Wry>` parameter and the `AppHandle::state()` method.
///
/// Note that it contains a `NodeManagerWorker`. This makes the desktop app a full-fledged node
/// with its own set of secure channels, outlets, transports etc...
///
/// However there is no associated persistence yet so outlets created with this `NodeManager` will
/// have to be recreated when the application restarts.
pub struct AppState {
    context: Arc<Context>,
    state: Arc<RwLock<CliState>>,
    node_manager: Arc<RwLock<Arc<InMemoryNode>>>,
    model_state: Arc<RwLock<ModelState>>,
    model_state_repository: Arc<RwLock<Arc<dyn ModelStateRepository>>>,
    event_manager: StdRwLock<EventManager>,
    background_node_client: Arc<RwLock<Arc<dyn BackgroundNodeClient>>>,

    #[cfg(debug_assertions)]
    browser_dev_tools: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create a new AppState
    pub fn new() -> AppState {
        let cli_state = CliState::initialize().unwrap_or_else(|_| {
            CliState::backup_and_reset().expect(
                "Failed to initialize CliState. Try to manually remove the '~/.ockam' directory",
            )
        });
        let (context, mut executor) = NodeBuilder::new().no_logging().build();
        let context = Arc::new(context);

        // from now on we use the same runtime everywhere we need to run an async action
        tauri::async_runtime::set(context.runtime().clone());

        // start the router, it is needed for the node manager creation
        spawn(async move { executor.start_router().await });
        let node_manager = Arc::new(create_node_manager(context.clone(), &cli_state));
        let model_state_repository = create_model_state_repository(&cli_state);
        let model_state = load_model_state(
            model_state_repository.clone(),
            node_manager.clone(),
            context.clone(),
            &cli_state,
        );

        info!("AppState initialized");

        AppState {
            context,
            state: Arc::new(RwLock::new(cli_state)),
            node_manager: Arc::new(RwLock::new(node_manager)),
            model_state: Arc::new(RwLock::new(model_state)),
            model_state_repository: Arc::new(RwLock::new(model_state_repository)),
            event_manager: StdRwLock::new(EventManager::new()),
            background_node_client: Arc::new(RwLock::new(Arc::new(Cli::new()))),

            #[cfg(debug_assertions)]
            browser_dev_tools: Default::default(),
        }
    }

    pub async fn reset(&self) -> miette::Result<()> {
        self.reset_state().await?;
        self.reset_node_manager().await?;

        {
            let mut writer = self.event_manager.write().unwrap();
            writer.events.clear();
        }

        // recreate the model state repository since the cli state has changed
        {
            let mut writer = self.model_state.write().await;
            *writer = ModelState::default();
        }
        let identity_path = self
            .state()
            .await
            .identities
            .identities_repository_path()
            .expect("Failed to get the identities repository path");
        let new_state_repository = LmdbModelStateRepository::new(identity_path).await?;
        {
            let mut writer = self.model_state_repository.write().await;
            *writer = Arc::new(new_state_repository);
        }

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
        let mut node_manager = self.node_manager.write().await;
        let _ = node_manager.stop(&self.context).await;
        info!("stopped the old node manager");

        for w in self.context.list_workers().await.into_diagnostic()? {
            let _ = self.context.stop_worker(w.address()).await;
        }
        info!("stopped all the ctx workers");

        let new_node_manager = make_node_manager(self.context.clone(), &self.state().await).await?;
        *node_manager = Arc::new(new_node_manager);
        info!("set a new node manager");
        Ok(())
    }

    /// Return the application Context
    /// This can be used to run async actions involving the Router
    pub fn context(&self) -> Arc<Context> {
        self.context.clone()
    }

    /// Return the application cli state
    /// This can be used to manage the on-disk state for projects, identities, vaults, etc...
    pub async fn state(&self) -> CliState {
        let state = self.state.read().await;
        state.clone()
    }

    /// Return the node manager
    pub async fn node_manager(&self) -> Arc<InMemoryNode> {
        let node_manager = self.node_manager.read().await;
        node_manager.clone()
    }

    /// Return a client to access the Controller
    pub async fn controller(&self) -> Result<Controller> {
        let node_manager = self.node_manager.read().await;
        Ok(node_manager.create_controller().await?)
    }

    pub async fn is_enrolled(&self) -> Result<bool> {
        self.state().await.is_enrolled().map_err(|e| {
            warn!(%e, "Failed to check if user is enrolled");
            e.into()
        })
    }

    /// Return the list of currently running outlets
    pub async fn tcp_outlet_list(&self) -> Vec<OutletStatus> {
        let node_manager = self.node_manager.read().await;
        node_manager.list_outlets().await.list
    }

    pub async fn user_info(&self) -> Result<UserInfo> {
        Ok(self
            .state
            .read()
            .await
            .users_info
            .default()?
            .config()
            .clone())
    }

    pub async fn user_email(&self) -> Result<String> {
        self.user_info().await.map(|u| u.email)
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

    /// Returns an EventDebouncer that will prevent the same event from being processed twice concurrently.
    ///
    /// The returned instance must be held in a variable until the event is processed. Once the variable
    /// is dropped, the event will be marked as "free".
    pub fn debounce_event<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        event_name: &str,
    ) -> EventDebouncer<R> {
        let is_processing = {
            let mut writer = self.event_manager.write().unwrap();
            writer.is_processing(event_name)
        };
        EventDebouncer {
            app: app.clone(),
            event_name: event_name.to_string(),
            is_processing,
        }
    }

    pub async fn background_node_client(&self) -> Arc<dyn BackgroundNodeClient> {
        self.background_node_client.read().await.clone()
    }
}

#[cfg(debug_assertions)]
impl AppState {
    pub fn browser_dev_tools(&self) -> bool {
        self.browser_dev_tools
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_browser_dev_tools(&self, value: bool) {
        self.browser_dev_tools
            .store(value, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn controller_address(&self) -> ockam_multiaddr::MultiAddr {
        use ockam_api::nodes::NodeManager;
        NodeManager::controller_multiaddr()
    }
}

/// Create a node manager
fn create_node_manager(ctx: Arc<Context>, cli_state: &CliState) -> InMemoryNode {
    match block_on(async { make_node_manager(ctx.clone(), cli_state).await }) {
        Ok(w) => w,
        Err(e) => {
            error!(%e, "Cannot load the model state");
            panic!("Cannot load the model state: {e:?}")
        }
    }
}

/// Make a node manager with a default node called "default"
pub(crate) async fn make_node_manager(
    ctx: Arc<Context>,
    cli_state: &CliState,
) -> miette::Result<InMemoryNode> {
    init_node_state(cli_state, NODE_NAME, None, None).await?;

    let tcp = TcpTransport::create(&ctx).await.into_diagnostic()?;
    let options = TcpListenerOptions::new();
    let listener = tcp
        .listen(&"127.0.0.1:0", options)
        .await
        .into_diagnostic()?;

    add_project_info_to_node_state(NODE_NAME, cli_state, None).await?;

    let node_state = cli_state.nodes.get(NODE_NAME)?;
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
    let trust_context_config = TrustContextConfigBuilder::new(cli_state).build();

    let node_manager = InMemoryNode::new(
        &ctx,
        NodeManagerGeneralOptions::new(cli_state.clone(), NODE_NAME.to_string(), None, true, true),
        NodeManagerTransportOptions::new(listener.flow_control_id().clone(), tcp),
        NodeManagerTrustOptions::new(trust_context_config),
    )
    .await
    .into_diagnostic()?;

    ctx.flow_controls()
        .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
    Ok(node_manager)
}

/// Create the repository containing the model state
fn create_model_state_repository(state: &CliState) -> Arc<dyn ModelStateRepository> {
    let identity_path = state
        .identities
        .identities_repository_path()
        .expect("Failed to get the identities repository path");
    match block_on(async move { LmdbModelStateRepository::new(identity_path).await }) {
        Ok(model_state_repository) => Arc::new(model_state_repository),
        Err(e) => {
            error!(%e, "Cannot create a model state repository manager");
            panic!("Cannot create a model state repository manager: {e:?}");
        }
    }
}

/// Load a previously persisted ModelState
fn load_model_state(
    model_state_repository: Arc<dyn ModelStateRepository>,
    node_manager: Arc<InMemoryNode>,
    context: Arc<Context>,
    cli_state: &CliState,
) -> ModelState {
    crate::shared_service::relay::load_model_state(
        context.clone(),
        node_manager.clone(),
        cli_state,
    );
    block_on(async {
        match model_state_repository.load().await {
            Ok(model_state) => {
                let model_state = model_state.unwrap_or(ModelState::default());
                crate::shared_service::tcp_outlet::load_model_state(
                    context.clone(),
                    node_manager.clone(),
                    &model_state,
                    cli_state,
                )
                .await;
                model_state
            }
            Err(e) => {
                error!(?e, "Cannot load the model state");
                panic!("Cannot load the model state: {e:?}")
            }
        }
    })
}

pub type EventName = String;
type IsProcessing = AtomicBool;
struct Event {
    name: EventName,
    is_processing: IsProcessing,
}

struct EventManager {
    events: Vec<Event>,
}

impl EventManager {
    fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Add a new event if it doesn't exist
    fn add(&mut self, event_name: &str) {
        if self.events.iter().any(|e| e.name == event_name) {
            return;
        }
        let event = Event {
            name: event_name.to_string(),
            is_processing: AtomicBool::new(true),
        };
        self.events.push(event);
        trace!(%event_name, "New event registered");
    }

    /// Check if it's being processed
    fn is_processing(&mut self, event_name: &str) -> bool {
        match self.events.iter().find(|e| e.name == event_name) {
            Some(e) => {
                let is_processing = e.is_processing.load(Ordering::SeqCst);
                if !is_processing {
                    e.is_processing.store(true, Ordering::SeqCst);
                }
                trace!(%event_name, is_processing, "Event status");
                is_processing
            }
            None => {
                self.add(event_name);
                false
            }
        }
    }

    /// Reset an event after it's been dropped
    fn reset(&self, event_name: &str, processed: bool) {
        if let Some(e) = self.events.iter().find(|e| e.name == event_name) {
            if processed {
                trace!(%event_name, "Event reset");
                e.is_processing.store(false, Ordering::SeqCst);
            }
        }
    }
}

pub struct EventDebouncer<R: Runtime> {
    app: AppHandle<R>,
    event_name: EventName,
    is_processing: bool,
}

impl<R: Runtime> EventDebouncer<R> {
    pub fn is_processing(&self) -> bool {
        self.is_processing
    }
}

impl<R: Runtime> Drop for EventDebouncer<R> {
    fn drop(&mut self) {
        let state = self.app.state::<AppState>();
        let reader = state.event_manager.read().unwrap();
        reader.reset(&self.event_name, !self.is_processing);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_manager() {
        let mut event_manager = EventManager::new();
        let event_name = "e1";

        // The first call using an unregistered event will register it and return false
        assert!(!event_manager.is_processing(event_name));

        // The second call will return true, as the event has not been marked as processed
        assert!(event_manager.is_processing(event_name));

        // Resetting the event marking it as unprocessed will leave the event as processing
        event_manager.reset(event_name, false);
        assert!(event_manager.is_processing(event_name));

        // Resetting the event marking it as processed will leave the event as processed
        event_manager.reset(event_name, true);

        // The event is now ready to get processed again
        assert!(!event_manager.is_processing(event_name));
    }
}
