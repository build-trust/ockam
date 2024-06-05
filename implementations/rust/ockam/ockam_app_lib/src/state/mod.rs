use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use miette::{IntoDiagnostic, WrapErr};
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tracing::{error, info, trace, warn};

pub use kind::StateKind;
use ockam::tcp::{TcpListenerOptions, TcpTransport};
use ockam::AsyncTryClone;
use ockam::Context;
use ockam::NodeBuilder;
use ockam_api::cli_state::CliState;
use ockam_api::cloud::enroll::auth0::UserInfo;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::{AuthorityNodeClient, ControllerClient};
use ockam_api::logs::TracingGuard;
use ockam_api::nodes::models::portal::OutletStatus;
use ockam_api::nodes::service::{NodeManagerGeneralOptions, NodeManagerTransportOptions};
use ockam_api::nodes::{BackgroundNodeClient, InMemoryNode, NodeManagerWorker, NODEMANAGER_ADDR};

use crate::api::notification::rust::{Notification, NotificationCallback};
use crate::api::state::rust::{
    ApplicationState, ApplicationStateCallback, Invitation, Invitee, LocalService, Service,
    ServiceGroup,
};
use crate::api::state::OrchestratorStatus;
use crate::background_node::{BackgroundNodeClientTrait, Cli};
use crate::incoming_services::IncomingServicesState;
use crate::invitations::state::{InvitationState, ReceivedInvitationStatus};
use crate::scheduler::Scheduler;
pub(crate) use crate::state::model::ModelState;
use crate::state::model_state_repository::ModelStateRepository;
pub(crate) use crate::state::model_state_repository_sql::ModelStateSqlxDatabase;
use crate::state::tasks::{
    RefreshInletsTask, RefreshInvitationsTask, RefreshProjectsTask, RefreshRelayTask,
};
use crate::{api, Result};

mod kind;
mod model;
mod model_state_repository;
mod model_state_repository_sql;
mod tasks;

pub const NODE_NAME: &str = "ockam_app";
// TODO: static project name of "default" is an unsafe default behavior due to backend uniqueness requirements
pub const PROJECT_NAME: &str = "default";

/// The AppState struct contains all the state managed by the application.
/// It can be retrieved with the `AppHandle<Wry>` parameter and the `AppHandle::state()` method.
///
/// Note that it contains a `NodeManagerWorker`. This makes the desktop app a full-fledged node
/// with its own set of secure channels, outlets, transports etc...
///
/// However there is no associated persistence yet so outlets created with this `NodeManager` will
/// have to be recreated when the application restarts.
#[derive(Clone)]
pub struct AppState {
    context: Arc<Context>,
    state: Arc<RwLock<CliState>>,
    orchestrator_status: Arc<Mutex<OrchestratorStatus>>,
    model_state: Arc<RwLock<ModelState>>,
    model_state_repository: Arc<RwLock<Arc<dyn ModelStateRepository>>>,
    background_node_client: Arc<RwLock<Arc<dyn BackgroundNodeClientTrait>>>,
    projects: Arc<RwLock<Vec<Project>>>,
    invitations: Arc<RwLock<InvitationState>>,
    incoming_services: Arc<RwLock<IncomingServicesState>>,
    application_state_callback: Option<ApplicationStateCallback>,
    notification_callback: Option<NotificationCallback>,
    node_manager: Arc<RwLock<Arc<InMemoryNode>>>,
    state_loaded: Arc<Mutex<u8>>,
    refresh_project_scheduler: Arc<OnceLock<Scheduler>>,
    refresh_invitations_scheduler: Arc<OnceLock<Scheduler>>,
    refresh_inlets_scheduler: Arc<OnceLock<Scheduler>>,
    refresh_relay_scheduler: Arc<OnceLock<Scheduler>>,
    last_published_snapshot: Arc<Mutex<Option<ApplicationState>>>,
    pub(crate) tracing_guard: Arc<OnceLock<TracingGuard>>,
}

async fn create_node_manager(ctx: Arc<Context>, cli_state: &CliState) -> Arc<InMemoryNode> {
    match make_node_manager(ctx.clone(), cli_state).await {
        Ok(w) => w,
        Err(e) => {
            error!(%e, "Cannot load the model state");
            panic!("Cannot load the model state: {e:?}")
        }
    }
}

impl AppState {
    /// Creates a new AppState, if it fails you can assume it's because the state cannot be loaded
    /// when `cli_state` is `None` the default initialization will be used
    pub fn new(
        application_state_callback: ApplicationStateCallback,
        notification_callback: NotificationCallback,
    ) -> Result<AppState> {
        let cli_state = CliState::with_default_dir()?;
        let rt = Arc::new(Runtime::new().expect("cannot create a tokio runtime"));
        let (context, mut executor) = NodeBuilder::new()
            .no_logging()
            .with_runtime(rt.clone())
            .build();
        let context = Arc::new(context);

        // start the router, it is needed for the node manager creation
        rt.spawn(async move {
            let result = executor.start_router().await;
            if let Err(e) = result {
                error!(%e, "Failed to start the router")
            }
        });

        let runtime = context.runtime().clone();
        let future = async {
            Self::make(
                context,
                Some(application_state_callback),
                Some(notification_callback),
                cli_state,
            )
            .await
        };

        Ok(runtime.block_on(future))
    }

    /// Creates a new AppState for testing purposes
    #[cfg(test)]
    pub async fn test(context: &Context, cli_state: CliState) -> AppState {
        let context = ockam_core::AsyncTryClone::async_try_clone(context)
            .await
            .unwrap();
        Self::make(Arc::new(context), None, None, cli_state).await
    }

    async fn make(
        context: Arc<Context>,
        application_state_callback: Option<ApplicationStateCallback>,
        notification_callback: Option<NotificationCallback>,
        cli_state: CliState,
    ) -> AppState {
        // create the application state and its dependencies
        let node_manager = create_node_manager(context.clone(), &cli_state).await;
        let model_state_repository = create_model_state_repository(&cli_state);
        let model_state = model_state_repository
            .load(&node_manager.node_name())
            .await
            .unwrap_or(ModelState::default());

        info!("AppState initialized");
        AppState {
            context,
            application_state_callback,
            notification_callback,
            state: Arc::new(RwLock::new(cli_state)),
            orchestrator_status: Arc::new(Mutex::new(Default::default())),
            node_manager: Arc::new(RwLock::new(node_manager)),
            model_state: Arc::new(RwLock::new(model_state)),
            model_state_repository: Arc::new(RwLock::new(model_state_repository)),
            background_node_client: Arc::new(RwLock::new(Arc::new(Cli::new()))),
            projects: Arc::new(Default::default()),
            invitations: Arc::new(RwLock::new(InvitationState::default())),
            incoming_services: Arc::new(RwLock::new(IncomingServicesState::default())),
            refresh_project_scheduler: Arc::new(Default::default()),
            refresh_invitations_scheduler: Arc::new(Default::default()),
            refresh_inlets_scheduler: Arc::new(Default::default()),
            refresh_relay_scheduler: Arc::new(Default::default()),
            last_published_snapshot: Arc::new(Mutex::new(None)),
            tracing_guard: Arc::new(Default::default()),
            state_loaded: Arc::new(Mutex::new(0)),
        }
    }

    /// Load a previously persisted ModelState and start refreshing schedule
    pub async fn load_model_state(&'static self) {
        self.restore_tcp_outlets().await;
        self.publish_state().await;

        let runtime = self.context.runtime();

        self.refresh_project_scheduler
            .set(Scheduler::create(
                Arc::new(RefreshProjectsTask::new(self.clone())),
                Duration::from_secs(30),
                runtime,
            ))
            .map_err(|_| "already set")
            .unwrap();

        self.refresh_invitations_scheduler
            .set(Scheduler::create(
                Arc::new(RefreshInvitationsTask::new(self.clone())),
                Duration::from_secs(30),
                runtime,
            ))
            .map_err(|_| "already set")
            .unwrap();

        self.refresh_inlets_scheduler
            .set(Scheduler::create(
                Arc::new(RefreshInletsTask::new(self.clone())),
                Duration::from_secs(10),
                runtime,
            ))
            .map_err(|_| "already set")
            .unwrap();

        self.refresh_relay_scheduler
            .set(Scheduler::create(
                Arc::new(RefreshRelayTask::new(self.clone())),
                Duration::from_secs(10),
                runtime,
            ))
            .map_err(|_| "already set")
            .unwrap();
    }

    /// Asynchronously shutdown the application
    pub fn shutdown(&self) {
        let context = self.context();
        let runtime = self.context.runtime().clone();

        let this = self.clone();
        runtime.spawn(async move {
            let result = this.node_manager.write().await.stop(&context).await;
            if let Err(e) = result {
                error!(?e, "Failed to shutdown the node manager")
            }
        });

        // delete every other app-related node, then exit
        let this = self.clone();
        runtime.spawn(async move {
            let inlets: Vec<String> = {
                let services = this.incoming_services().read().await.clone();
                services
                    .services
                    .iter()
                    .map(|inlet| inlet.local_node_name())
                    .collect()
            };

            for node_name in inlets.into_iter() {
                let _ = this.delete_background_node(&node_name).await;
            }

            std::process::exit(0);
        });
    }

    /// Starts the refresh of projects without waiting for the scheduler
    #[allow(dead_code)]
    pub fn schedule_projects_refresh_now(&self) {
        if let Some(scheduler) = self.refresh_project_scheduler.get() {
            scheduler.schedule_now();
        }
    }

    /// Starts the refresh of invitations without waiting for the scheduler
    pub fn schedule_invitations_refresh_now(&self) {
        if let Some(scheduler) = self.refresh_invitations_scheduler.get() {
            scheduler.schedule_now();
        }
    }

    /// Starts the refresh of relay without waiting for the scheduler
    pub fn schedule_relay_refresh_now(&self) {
        if let Some(scheduler) = self.refresh_relay_scheduler.get() {
            scheduler.schedule_now();
        }
    }

    /// Starts the refresh of inlets without waiting for the scheduler
    pub fn schedule_inlets_refresh_now(&self) {
        if let Some(scheduler) = self.refresh_inlets_scheduler.get() {
            scheduler.schedule_now();
        }
    }

    pub async fn reset(&self) -> miette::Result<()> {
        self.reset_state().await?;
        self.reset_node_manager().await?;

        // recreate the model state repository since the cli state has changed
        {
            let mut writer = self.model_state.write().await;
            *writer = ModelState::default();
        }
        let cli_state = &self.state().await;
        let new_state_repository = create_model_state_repository(cli_state);
        {
            let mut writer = self.model_state_repository.write().await;
            *writer = new_state_repository;
        }
        self.update_orchestrator_status(OrchestratorStatus::default());
        self.publish_state().await;

        Ok(())
    }

    async fn reset_state(&self) -> miette::Result<()> {
        let mut state = self.state.write().await;
        match state.recreate().await {
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
        node_manager
            .stop(&self.context)
            .await
            .into_diagnostic()
            .wrap_err("Failed to stop the node manager")?;

        info!("stopped the old node manager");

        for w in self.context.list_workers().await.into_diagnostic()? {
            let _ = self.context.stop_worker(w.address()).await;
        }
        info!("stopped all the ctx workers");

        let new_node_manager = make_node_manager(self.context.clone(), &self.state().await).await?;
        *node_manager = new_node_manager;
        info!("set a new node manager");
        Ok(())
    }

    /// Return the application Context
    /// This can be used to run async actions involving the Router
    pub fn context(&self) -> Arc<Context> {
        self.context.clone()
    }

    /// Return Context without cloning
    pub fn context_ref(&self) -> &Context {
        &self.context
    }

    /// Returns the list of projects
    pub fn projects(&self) -> Arc<RwLock<Vec<Project>>> {
        self.projects.clone()
    }

    /// Returns the status of invitations
    pub fn invitations(&self) -> Arc<RwLock<InvitationState>> {
        self.invitations.clone()
    }

    /// Returns the status of the services
    pub fn incoming_services(&self) -> Arc<RwLock<IncomingServicesState>> {
        self.incoming_services.clone()
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
    pub async fn controller(&self) -> Result<ControllerClient> {
        let node_manager = self.node_manager.read().await;
        Ok(node_manager.create_controller().await?)
    }

    pub async fn authority_node(
        &self,
        project: &Project,
        caller_identity_name: Option<String>,
    ) -> Result<AuthorityNodeClient> {
        let node_manager = self.node_manager.read().await;
        Ok(node_manager
            .create_authority_client(project, caller_identity_name)
            .await?)
    }

    pub async fn background_node(&self, node_name: &str) -> Result<BackgroundNodeClient> {
        let tcp = self
            .node_manager
            .read()
            .await
            .tcp_transport()
            .async_try_clone()
            .await?;
        Ok(
            BackgroundNodeClient::create_to_node_with_tcp(&tcp, &self.state().await, node_name)
                .await?,
        )
    }

    pub async fn backup_logs(&self, node_name: &str) -> Result<()> {
        Ok(self.state().await.backup_logs(node_name)?)
    }

    pub async fn delete_background_node(&self, node_name: &str) -> Result<()> {
        Ok(self.state().await.delete_node(node_name, true).await?)
    }

    pub async fn is_enrolled(&self) -> Result<bool> {
        Ok(self.state().await.is_enrolled().await.map_err(|e| {
            warn!(%e, "Failed to check if user is enrolled");
            e
        })?)
    }

    /// Return the list of currently running outlets
    pub async fn tcp_outlet_list(&self) -> Vec<OutletStatus> {
        let node_manager = self.node_manager.read().await;
        node_manager.list_outlets().await
    }

    pub async fn user_info(&self) -> Result<UserInfo> {
        Ok(self.state.read().await.get_default_user().await?)
    }

    pub async fn model_mut(&self, f: impl FnOnce(&mut ModelState)) -> Result<()> {
        let mut model_state = self.model_state.write().await;
        trace!(?model_state, "updating model state locally");
        f(&mut model_state);
        trace!(?model_state, "persisting model state to DB");

        let node_manager = self.node_manager.read().await;
        self.model_state_repository
            .read()
            .await
            .store(&node_manager.node_name(), &model_state)
            .await?;
        Ok(())
    }

    pub async fn model<T>(&self, f: impl FnOnce(&ModelState) -> T) -> T {
        let model_state = self.model_state.read().await;
        f(&model_state)
    }

    pub async fn background_node_client(&self) -> Arc<dyn BackgroundNodeClientTrait> {
        self.background_node_client.read().await.clone()
    }
    pub fn orchestrator_status(&self) -> OrchestratorStatus {
        self.orchestrator_status.lock().unwrap().clone()
    }

    pub fn update_orchestrator_status(&self, status: OrchestratorStatus) {
        *self.orchestrator_status.lock().unwrap() = status;
    }

    /// Update to the provided status only if the current status is within the provide statuses
    pub fn update_orchestrator_status_if(
        &self,
        status: OrchestratorStatus,
        statuses: Vec<OrchestratorStatus>,
    ) {
        let mut guard = self.orchestrator_status.lock().unwrap();
        if statuses.contains(&*guard) {
            *guard = status;
        }
    }

    pub fn notify(&self, notification: Notification) {
        if let Some(callback) = self.notification_callback.as_ref() {
            callback.call(notification);
        }
    }

    /// Sends the new application state to the UI
    pub async fn publish_state(&self) {
        if let Some(callback) = self.application_state_callback.as_ref() {
            let result = self.snapshot().await;
            match result {
                Ok(state) => {
                    {
                        // avoid publishing the same state multiple times
                        let mut guard = self.last_published_snapshot.lock().unwrap();
                        if let Some(previous) = &*guard {
                            if previous == &state {
                                return;
                            }
                        }
                        guard.replace(state.clone());
                    }
                    callback.call(state);
                }
                Err(e) => {
                    warn!(%e, "Failed to publish the application state");
                }
            }
        }
    }

    /// Creates a snapshot of the application state without any side-effects
    pub async fn snapshot(&self) -> Result<api::state::rust::ApplicationState> {
        let enrolled = self.is_enrolled().await.unwrap_or(false);
        let loaded;
        let orchestrator_status = self.orchestrator_status();
        let enrollment_name;
        let enrollment_email;
        let enrollment_image;
        let enrollment_github_user;
        let mut local_services: Vec<LocalService>;
        let mut groups: Vec<ServiceGroup>;
        let mut sent_invitations: Vec<Invitee>;
        let invitation_state = { self.invitations().read().await.clone() };
        let incoming_services_state = { self.incoming_services().read().await.clone() };

        // we want to sort everything to avoid having to deal with ordering in the UI
        if enrolled {
            loaded = self.is_state_loaded();
            local_services = self
                .tcp_outlet_list()
                .await
                .into_iter()
                .map(|outlet| LocalService {
                    name: outlet.worker_addr.address().to_string(),
                    address: outlet.socket_addr.ip().to_string(),
                    port: outlet.socket_addr.port(),
                    scheme: None,
                    shared_with: vec![],
                    available: true,
                })
                .collect();

            local_services.sort();
            let mut group_names = Vec::new();

            sent_invitations = invitation_state
                .sent
                .iter()
                .map(|invitation| Invitee {
                    name: Some(invitation.recipient_email.clone().to_string()),
                    email: invitation.recipient_email.clone(),
                })
                .collect();
            sent_invitations.sort();

            invitation_state
                .accepted
                .invitations
                .iter()
                .for_each(|invitation| {
                    if !group_names
                        .iter()
                        .any(|name| name == &invitation.invitation.owner_email)
                    {
                        group_names.push(invitation.invitation.owner_email.clone());
                    }
                });

            invitation_state
                .received
                .invitations
                .iter()
                .for_each(|invitation| {
                    if !group_names
                        .iter()
                        .any(|name| name == &invitation.owner_email)
                    {
                        group_names.push(invitation.owner_email.clone());
                    }
                });

            group_names.sort();

            groups = group_names
                .into_iter()
                .map(|email| ServiceGroup {
                    email: email.clone(),
                    name: None,
                    image_url: None,
                    invitations: {
                        let mut invitations: Vec<Invitation> = invitation_state
                            .received
                            .invitations
                            .iter()
                            .filter(|invitation| invitation.owner_email == email)
                            .map(|invitation| Invitation {
                                id: invitation.id.clone(),
                                service_name: {
                                    let mut name = invitation.id.clone();
                                    name.truncate(6);
                                    name
                                },
                                service_scheme: None,
                                accepting: invitation_state
                                    .received
                                    .status
                                    .iter()
                                    .find(|(id, _)| id == &invitation.id)
                                    .map(|(_, status)| {
                                        status == &ReceivedInvitationStatus::Accepting
                                    })
                                    .unwrap_or(false),
                                accepted: invitation_state
                                    .received
                                    .status
                                    .iter()
                                    .find(|(id, _)| id == &invitation.id)
                                    .map(|(_, status)| {
                                        status == &ReceivedInvitationStatus::Accepted
                                    })
                                    .unwrap_or(false),
                                ignoring: invitation_state
                                    .received
                                    .status
                                    .iter()
                                    .find(|(id, _)| id == &invitation.id)
                                    .map(|(_, status)| {
                                        status == &ReceivedInvitationStatus::Ignoring
                                    })
                                    .unwrap_or(false),
                            })
                            .collect();

                        invitations.sort();
                        invitations
                    },
                    incoming_services: {
                        let mut incoming_services: Vec<Service> = incoming_services_state
                            .services
                            .iter()
                            .filter(|service| service.email() == &email)
                            .map(|service| Service {
                                id: service.id().to_string(),
                                source_name: service.name().to_string(),
                                address: service.address().map(|addr| addr.ip().to_string()),
                                port: service.port(),
                                scheme: None,
                                available: service.is_connected(),
                                enabled: service.enabled(),
                            })
                            .collect();

                        incoming_services.sort();
                        incoming_services
                    },
                })
                .collect();
            groups.sort();

            let user_info = self.user_info().await?;
            // when enrolling with email, the name is just a duplicate of the
            // email, in case case it's better to just omit the name
            if user_info.name == user_info.email.to_string() {
                enrollment_name = None;
            } else {
                enrollment_name = Some(user_info.name);
            }
            enrollment_email = Some(user_info.email);
            enrollment_image = Some(user_info.picture);
            enrollment_github_user = Some(user_info.nickname);
        } else {
            loaded = false;
            enrollment_name = None;
            enrollment_email = None;
            enrollment_image = None;
            enrollment_github_user = None;
            local_services = vec![];
            groups = vec![];
            sent_invitations = vec![];
        }

        Ok(ApplicationState {
            enrolled,
            loaded,
            orchestrator_status,
            enrollment_name,
            enrollment_email,
            enrollment_image,
            enrollment_github_user,
            local_services,
            groups,
            sent_invitations,
        })
    }
}

/// Make a node manager with a default node called "default"
pub(crate) async fn make_node_manager(
    ctx: Arc<Context>,
    cli_state: &CliState,
) -> miette::Result<Arc<InMemoryNode>> {
    let tcp = TcpTransport::create(&ctx).await.into_diagnostic()?;
    let options = TcpListenerOptions::new();
    let listener = tcp
        .listen(&"127.0.0.1:0", options)
        .await
        .into_diagnostic()?;

    let _ = cli_state
        .start_node_with_optional_values(NODE_NAME, &None, &None, Some(&listener))
        .await?;

    let trust_options = cli_state
        .retrieve_trust_options(&None, &None, &None, &None)
        .await
        .into_diagnostic()?;

    let node_manager = Arc::new(
        InMemoryNode::new(
            &ctx,
            NodeManagerGeneralOptions::new(
                cli_state.clone(),
                NODE_NAME.to_string(),
                true,
                None,
                true,
            ),
            NodeManagerTransportOptions::new(listener.flow_control_id().clone(), tcp),
            trust_options,
        )
        .await
        .into_diagnostic()?,
    );

    let node_manager_worker = NodeManagerWorker::new(node_manager.clone());
    ctx.start_worker(NODEMANAGER_ADDR, node_manager_worker)
        .await
        .into_diagnostic()?;

    ctx.flow_controls()
        .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
    Ok(node_manager)
}

/// Create the repository containing the model state
fn create_model_state_repository(state: &CliState) -> Arc<dyn ModelStateRepository> {
    Arc::new(ModelStateSqlxDatabase::new(state.database()))
}
