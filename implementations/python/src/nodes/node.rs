use super::{PyMailbox, PyWorker, Spawner, get_runtime, get_runtime_ref};

use crate::errors::py_error;
use crate::integrations::mcp;
use crate::integrations::mcp::ExposedWorkers;
use crate::nodes::StartedAgents;
use crate::nodes::StartedWorkers;
use ockam::abac::{Expr, PolicyExpression, message_is_local_policy_expression};
use ockam::compat::asynchronous::RwLock;
use ockam::compat::sync::Arc;
use ockam::env::get_env_with_default;
use ockam::identity::Identifier;
use ockam::tcp::{TcpListenerOptions, TcpTransport};
use ockam::{
    Address, Context, Executor, Mailboxes, MessageSendReceiveOptions, NodeBuilder,
    OCKAM_DATABASE_USER, OCKAM_SQLITE_IN_MEMORY, Result, Route, TryClone, WorkerBuilder,
};
use ockam_api::cli_state::{EnrollmentTicket, ExportedEnrollmentTicket, NamedIdentity};
use ockam_api::enroll::enrollment::{EnrollStatus, Enrollment};
use ockam_api::multiaddr::MultiAddr;
use ockam_api::nodes::models::relay::ReturnTiming;
use ockam_api::nodes::service::{
    NodeManager, NodeManagerGeneralOptions, NodeManagerTransport, NodeManagerTransportOptions,
};
use ockam_api::nodes::{InMemoryNode, InMemoryNodeBuilder, NodeManagerDefaults};
use ockam_api::orchestrator::AuthorityNodeClient;
use ockam_api::orchestrator::project::Project;
use ockam_api::{ApiError, CliState};

use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;

use miette::{IntoDiagnostic, miette};

use crate::nodes::logging::py_debug;
use crate::nodes::runtime::PythonAsyncExecutor;
use ockam::access_control::AllowAll;
use serde::Serialize;
use serde_json::Value;
use serde_pyobject::to_pyobject;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tracing::{error, info};

#[pyclass(name = "Node")]
#[derive(Clone)]
pub struct PyNode {
    #[pyo3(get)]
    name: String,
    node_manager: Arc<NodeManager>,
    node: Arc<InMemoryNode>,
    exposed_workers: ExposedWorkers,
    started_agents: StartedAgents,
    started_workers: StartedWorkers,
    #[allow(dead_code)]
    executor: Executor,
    project: Option<Project>,
    default_policy: Expr,

    #[allow(clippy::type_complexity)]
    remote_connection_cache: Option<Arc<RwLock<BTreeMap<(String, String), CachedRemoteRoute>>>>,
}

struct CachedRemoteRoute {
    last_used: Instant,
    route: Route,
}

impl PyNode {
    pub fn node_manager(&self) -> &NodeManager {
        &self.node_manager
    }

    pub fn node_manager_clone(&self) -> Arc<NodeManager> {
        self.node_manager.clone()
    }

    /// Return the list of the workers started by the node:
    /// - the workers started by the node itself via some user code
    /// - the workers started by the agents
    ///
    /// The workers that are started to support the node functionality, like the RemoteManager worker
    /// are not included in this list.
    pub async fn list_workers_as_json(&self) -> Value {
        let mut result = vec![];
        let agent_workers = self.started_agents.list_agents().await;
        for agent in agent_workers {
            for worker in agent.workers {
                result.push(WorkerOutput::new(worker, Some(agent.name.clone())));
            }
        }

        let workers = self.started_workers.list_workers().await;
        // Skip internal workers
        for worker in workers {
            if !worker.is_internal() {
                result.push(WorkerOutput::new(worker.name, None));
            }
        }
        serde_json::to_value(result).unwrap_or_else(|_| Value::Array(vec![]))
    }
}

#[derive(Debug, Clone, Serialize)]
struct WorkerOutput {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_by_agent: Option<String>,
}

impl WorkerOutput {
    pub fn new(name: String, started_by_agent: Option<String>) -> Self {
        Self {
            name,
            started_by_agent,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum WorkerType {
    UserWorker,
    InternalWorker,
}

#[pymethods]
impl PyNode {
    #[getter]
    fn is_remote(&self) -> bool {
        false
    }

    #[staticmethod]
    #[pyo3(signature = (main, name=None, ticket=None, mcp_server=None, mcp_clients=None, allow=None, cache_secure_channels=false))]
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        py: Python<'_>,
        main: PyObject,
        name: Option<String>,
        ticket: Option<String>,
        mcp_server: Option<PyObject>,
        mcp_clients: Option<Vec<PyObject>>,
        allow: Option<String>,
        cache_secure_channels: bool,
    ) -> PyResult<PyObject> {
        let _ = pyo3_async_runtimes::tokio::init_with_runtime(get_runtime_ref());

        let node = get_runtime().block_on(async move {
            let node_builder = NodeBuilder::new()
                .with_runtime(get_runtime())
                .with_logging(false);
            let (ctx, executor) = node_builder.build();

            let node = if let Some(ticket) = ticket {
                PyNode::create_with_ticket(
                    &ctx,
                    executor,
                    ticket.as_str(),
                    allow,
                    name,
                    cache_secure_channels,
                )
                .await
                .map_err(py_error)?
            } else {
                PyNode::create(&ctx, executor, allow, name, cache_secure_channels)
                    .await
                    .map_err(py_error)?
            };

            if let Some(server) = mcp_server {
                let listen_address = {
                    let result = server
                        .getattr(py, "listen_address")
                        .expect("could not extract listen_address");
                    let address: String = result
                        .extract(py)
                        .expect("could not extract listen_address");

                    address
                };

                mcp::start_server(node.clone(), node.exposed_workers.clone(), &listen_address)
                    .await
                    .map_err(py_error)?;
            }

            if let Some(clients) = mcp_clients {
                let mut client_addresses = Vec::new();
                for client in clients {
                    let client_address = {
                        let result = client.getattr(py, "name").expect("could not extract name");
                        let name: String = result.extract(py).expect("could not extract name");

                        let result = client
                            .getattr(py, "address")
                            .expect("could not extract address");
                        let address: String =
                            result.extract(py).expect("could not extract address");

                        (name, address)
                    };
                    client_addresses.push(client_address);
                }

                mcp::start_clients(&node, client_addresses)
                    .await
                    .map_err(py_error)?;
            }

            Ok::<PyNode, PyErr>(node)
        })?;

        let py_future = main.call1(py, (node.clone(),))?;
        let res = PythonAsyncExecutor::run_python_main(py, py_future)?;

        node.stop()?;

        Ok(res)
    }

    #[pyo3(signature = (name, py_object, policy=None, exposed_as=None))]
    fn start_worker<'a>(
        &self,
        py: Python<'a>,
        name: &str,
        py_object: PyObject,
        policy: Option<String>,
        exposed_as: Option<String>,
    ) -> PyResult<Bound<'a, PyAny>> {
        self.start_worker_impl(py, name, py_object, WorkerKind::User, policy, exposed_as)
    }

    #[pyo3(signature = (name, py_object, policy=None, exposed_as=None))]
    fn start_internal_worker<'a>(
        &self,
        py: Python<'a>,
        name: &str,
        py_object: PyObject,
        policy: Option<String>,
        exposed_as: Option<String>,
    ) -> PyResult<Bound<'a, PyAny>> {
        self.start_worker_impl(
            py,
            name,
            py_object,
            WorkerKind::Internal,
            policy,
            exposed_as,
        )
    }

    #[pyo3(signature = (name, py_factory, key_extractor, policy=None, exposed_as=None))]
    fn start_spawner<'a>(
        &self,
        py: Python<'a>,
        name: &str,
        py_factory: PyObject,
        key_extractor: PyObject,
        policy: Option<String>,
        exposed_as: Option<String>,
    ) -> PyResult<Bound<'a, PyAny>> {
        self.start_worker_impl(
            py,
            name,
            py_factory,
            WorkerKind::Spawner { key_extractor },
            policy,
            exposed_as,
        )
    }

    #[pyo3(signature = (name))]
    fn stop_worker<'a>(&self, py: Python<'a>, name: &str) -> PyResult<Bound<'a, PyAny>> {
        let address: Address = name.to_string().into();
        self.ctx().stop_address(&address).map_err(py_error)?;

        let started_agents = self.started_agents.clone();
        let started_workers = self.started_workers.clone();
        let name = name.to_string();

        future_into_py(py, async move {
            started_agents.stopped(&name).await;
            started_workers.stopped(&name).await;
            Ok(())
        })
    }

    #[pyo3(signature = ())]
    fn list_nodes_priv<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let self_clone = self.clone();
        future_into_py(py, async move {
            let project = self_clone
                .project
                .as_ref()
                .ok_or(py_error("Not member on any zone"))?;
            let project_client = self_clone
                .node_manager_clone()
                .create_project_client(
                    &project
                        .project_identifier()
                        .ok_or(py_error("Error resolving zone"))?,
                    project.project_multiaddr().map_err(py_error)?,
                    None,
                    ockam_api::orchestrator::CredentialsEnabled::On,
                )
                .await
                .map_err(py_error)?;
            let relays = project_client
                .list_relays(self_clone.ctx())
                .await
                .map_err(py_error)?;
            let relay_addrs: Vec<String> = relays.into_iter().map(|r| r.addr).collect();
            Ok(relay_addrs)
        })
    }

    #[pyo3(signature = (destination, message, policy=None, timeout=None))]
    fn send_and_receive<'a>(
        &self,
        py: Python<'a>,
        destination: String,
        message: String,
        #[allow(unused_variables)] policy: Option<String>,
        timeout: Option<u64>,
    ) -> PyResult<Bound<'a, PyAny>> {
        self.send_and_receive_impl(py, None, destination, message, policy, timeout)
    }

    #[pyo3(signature = (node, destination, message, policy=None, timeout=None))]
    fn send_and_receive_to_remote<'a>(
        &self,
        py: Python<'a>,
        node: String,
        destination: String,
        message: String,
        #[allow(unused_variables)] policy: Option<String>,
        timeout: Option<u64>,
    ) -> PyResult<Bound<'a, PyAny>> {
        self.send_and_receive_impl(py, Some(node), destination, message, policy, timeout)
    }

    #[pyo3(signature=(address, policy=None))]
    fn create_mailbox<'a>(
        &self,
        py: Python<'a>,
        address: String,
        #[allow(unused_variables)] policy: Option<String>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let context = self.ctx().get_router_context();
        let self_clone = self.clone();

        future_into_py(py, async move {
            let (incoming_ac, outgoing_ac) = (Arc::new(AllowAll), Arc::new(AllowAll));

            let address = Address::from(address);
            let mailboxes = Mailboxes::primary(address, incoming_ac, outgoing_ac);
            let context = context
                .new_detached_with_mailboxes(mailboxes)
                .map_err(py_error)?;
            let py_mailbox = PyMailbox::new(context, self_clone);

            Ok(py_mailbox)
        })
    }

    fn mcp_tools<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let node = self.clone();

        future_into_py(py, async move {
            let r = mcp::list_tools(&node).await.map_err(py_error)?;

            Ok(r)
        })
    }

    fn call_mcp_tool<'a>(
        &self,
        py: Python<'a>,
        server_name: String,
        tool_name: String,
        tool_args_as_json: Option<String>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let node = self.clone();

        future_into_py(py, async move {
            let r = mcp::call_tool(&node, server_name, tool_name, tool_args_as_json)
                .await
                .map_err(py_error)?;

            Ok(r)
        })
    }

    fn mcp_tool_spec<'a>(
        &self,
        py: Python<'a>,
        server_name: String,
        tool_name: String,
    ) -> PyResult<Bound<'a, PyAny>> {
        let node = self.clone();

        future_into_py(py, async move {
            let r = mcp::tool_spec(&node, server_name, tool_name)
                .await
                .map_err(py_error)?;

            Ok(r)
        })
    }

    fn stop(&self) -> PyResult<()> {
        get_runtime().block_on(async {
            let res1 = self.node.stop().await.map_err(py_error);
            let res2 = self.ctx().shutdown_node().await.map_err(py_error);

            res1?;
            res2?;

            Ok(())
        })
    }

    fn interrupted<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let context = self.ctx().try_clone().map_err(py_error)?;

        future_into_py(py, async move {
            _ = ockam::compat::tokio::signal::ctrl_c().await;
            _ = context.shutdown_node().await;

            Ok(())
        })
    }

    pub fn identifier(&self, py: Python<'_>) -> PyResult<PyObject> {
        let identifier = self.node_manager.identifier().to_string();
        to_pyobject(py, &identifier)
            .map(|v| v.unbind())
            .map_err(py_error)
    }

    pub fn list_agents<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let started_agents = self.started_agents.clone();
        future_into_py(py, async move {
            let agents = started_agents.list_agents_as_json().await;
            Python::with_gil(|py| {
                to_pyobject(py, &agents)
                    .map(|v| v.unbind())
                    .map_err(py_error)
            })
        })
    }

    pub fn list_workers<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let node = self.clone();
        future_into_py(py, async move {
            let workers = node.list_workers_as_json().await;
            Python::with_gil(|py| {
                to_pyobject(py, &workers)
                    .map(|v| v.unbind())
                    .map_err(py_error)
            })
        })
    }

    pub fn list_tools<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let node = self.clone();
        let tools = node.exposed_workers.tools_as_json();
        to_pyobject(py, &tools).map_err(py_error)
    }
}

#[derive(Debug)]
pub enum WorkerKind {
    User,
    Internal,
    Spawner { key_extractor: PyObject },
}

impl PyNode {
    async fn create(
        ctx: &Context,
        executor: Executor,
        default_policy: Option<String>,
        node_name: Option<String>,
        cache_secure_channels: bool,
    ) -> miette::Result<Self> {
        let (name, cli_state) = PyNode::initialize_state(&node_name).await?;
        let node = InMemoryNodeBuilder::create(ctx, cli_state)?
            .with_timeout(Some(Duration::from_millis(2000)))
            .start()
            .await?;

        let default_policy = if let Some(default_policy) = default_policy {
            PolicyExpression::from_str(default_policy.as_str())
                .map_err(ockam::Error::from)?
                .to_expression()
        } else {
            message_is_local_policy_expression()
        };

        Ok(PyNode {
            name,
            node_manager: node.inner_clone(),
            node,
            exposed_workers: ExposedWorkers::default(),
            started_agents: StartedAgents::default(),
            started_workers: StartedWorkers::default(),
            executor,
            project: None,
            default_policy,
            remote_connection_cache: cache_secure_channels
                .then(|| Arc::new(RwLock::new(BTreeMap::new()))),
        })
    }

    async fn create_with_ticket(
        ctx: &Context,
        executor: Executor,
        ticket: &str,
        default_policy: Option<String>,
        node_name: Option<String>,
        cache_secure_channels: bool,
    ) -> miette::Result<Self> {
        let default_policy = if let Some(default_policy) = default_policy {
            PolicyExpression::from_str(default_policy.as_str())
                .map_err(ockam::Error::from)?
                .to_expression()
        } else {
            message_is_local_policy_expression()
        };

        let (name, cli_state) = PyNode::initialize_state(&node_name).await?;
        let ticket = ExportedEnrollmentTicket::from_str(ticket)?.import().await?;
        let project = cli_state
            .projects()
            .import_and_store_project(ticket.project()?)
            .await?;

        let node_name = Self::get_node_name(&node_name)?;
        let named_identity = cli_state.get_named_identity(&node_name).await?;
        let authority_client = Self::create_authority_client(
            cli_state.clone(),
            ctx,
            project.clone(),
            &named_identity.identifier(),
        )
        .await?;
        Self::enroll(ctx, &authority_client, ticket).await?;

        let node =
            Self::create_in_memory_node(cli_state.clone(), ctx, project.name(), &node_name).await?;
        let node_manager = node.inner_clone();
        let _relay_name =
            Self::create_relay(ctx, node_manager.clone(), &authority_client, project.name())
                .await?;

        Ok(PyNode {
            name,
            node_manager,
            node,
            executor,
            exposed_workers: ExposedWorkers::default(),
            started_agents: StartedAgents::default(),
            started_workers: StartedWorkers::default(),
            project: Some(project),
            default_policy,
            remote_connection_cache: cache_secure_channels
                .then(|| Arc::new(RwLock::new(BTreeMap::new()))),
        })
    }

    /// We initialize the state with a node and an identity having the same name
    /// retrieved from the OCKAM_DATABASE_USER environment variable.
    async fn initialize_state(name: &Option<String>) -> miette::Result<(String, Arc<CliState>)> {
        let state =
            Arc::new(CliState::new(get_env_with_default(OCKAM_SQLITE_IN_MEMORY, false)?).await?);
        let node_name = Self::get_node_name(name)?;
        if state.get_node(&node_name).await.is_err() {
            let identity = Self::create_identity(state.clone(), &node_name).await?;
            state
                .create_node_with_identifier(&node_name, &identity.identifier())
                .await?;
        };
        Ok((node_name, state))
    }

    fn ctx(&self) -> &Context {
        self.node_manager.tcp_transport().ctx()
    }

    /// We use the database user as the node _name_ and the node identity name
    fn get_node_name(name: &Option<String>) -> miette::Result<String> {
        Ok(get_env_with_default(
            OCKAM_DATABASE_USER,
            name.clone().unwrap_or("default-node".to_string()),
        )?)
    }

    async fn create_identity(
        state: Arc<CliState>,
        identity_name: &str,
    ) -> miette::Result<NamedIdentity> {
        if let Ok(named_identity) = state.get_named_identity(identity_name).await {
            Ok(named_identity)
        } else {
            state
                .create_identity_with_name(identity_name)
                .await
                .into_diagnostic()
        }
    }

    async fn create_authority_client(
        state: Arc<CliState>,
        ctx: &Context,
        project: Project,
        caller: &Identifier,
    ) -> miette::Result<AuthorityNodeClient> {
        let tcp = TcpTransport::get_or_create(ctx)?;
        let authority_identifier = project
            .authority_identifier()
            .ok_or_else(|| ApiError::core("no authority identifier"))
            .into_diagnostic()?;
        NodeManager::authority_node_client(
            tcp.clone(),
            state.secure_channels().await?.clone(),
            &authority_identifier,
            project.authority_multiaddr().into_diagnostic()?,
            caller,
            None,
        )
        .await
        .into_diagnostic()
    }

    async fn enroll(
        ctx: &Context,
        authority_client: &AuthorityNodeClient,
        ticket: EnrollmentTicket,
    ) -> miette::Result<()> {
        let enroll_status = authority_client
            .present_token(ctx, &ticket.one_time_code)
            .await?;

        match enroll_status {
            EnrollStatus::EnrolledSuccessfully | EnrollStatus::AlreadyEnrolled => {
                info!("Enrolled successfully");
                Ok(())
            }

            EnrollStatus::UnexpectedStatus(str, status) => {
                Err(miette!("Unexpected Enroll status: {}, {}", str, status))
            }
            EnrollStatus::FailedNoStatus(str) => Err(miette!("Enrollment failed: {}", str)),
        }
    }

    async fn get_relay_name(
        ctx: &Context,
        authority_client: &AuthorityNodeClient,
    ) -> miette::Result<String> {
        let subject_attributes = authority_client.get_subject_attributes(ctx).await?;
        let relay_name = match subject_attributes
            .get("ockam-relay".as_bytes())
            .ok_or_else(|| {
                miette!("The ockam-relay attribute must be present in the enrollment ticket")
            }) {
            Ok(relay_name) => {
                String::from_utf8(relay_name.as_slice().to_vec()).into_diagnostic()?
            }
            Err(e) => {
                error!("{e}");
                return Err(e);
            }
        };
        Ok(relay_name)
    }

    async fn create_in_memory_node(
        state: Arc<CliState>,
        ctx: &Context,
        project_name: &str,
        node_name: &str,
    ) -> miette::Result<Arc<InMemoryNode>> {
        let tcp = TcpTransport::get_or_create(ctx)?;
        let tcp_listener = tcp
            .listen(
                NodeManagerDefaults::default().tcp_listener_address.as_str(),
                TcpListenerOptions::new(),
            )
            .await?;

        let node = state
            .start_node_with_optional_values(
                node_name,
                &Some(node_name.to_string()),
                Some(&tcp_listener),
            )
            .await?;

        let trust_options = state
            .retrieve_trust_options(&Some(project_name.to_string()), &None, &None, &None)
            .await?;

        let in_memory_node = InMemoryNode::new(
            ctx,
            NodeManagerGeneralOptions::new(state.clone(), node.name(), true, None, true),
            NodeManagerTransportOptions::new(
                NodeManagerTransport::new(tcp_listener.flow_control_id().clone(), tcp),
                None,
            ),
            trust_options,
        )
        .await?
        .with_timeout(Duration::from_millis(2000));

        Ok(Arc::new(in_memory_node))
    }

    async fn create_relay(
        ctx: &Context,
        node_manager: Arc<NodeManager>,
        authority_client: &AuthorityNodeClient,
        project_name: &str,
    ) -> miette::Result<String> {
        let relay_name = Self::get_relay_name(ctx, authority_client).await?;
        let _relay_info = node_manager
            .create_relay(
                &MultiAddr::from_str(&format!("/project/{}", project_name))?,
                relay_name.clone(),
                None,
                Some(relay_name.clone()),
                ReturnTiming::AfterConnection,
            )
            .await?;
        info!("Successfully created a relay for the node");
        Ok(relay_name)
    }

    fn start_worker_impl<'a>(
        &self,
        py: Python<'a>,
        name: &str,
        py_worker_or_factory: PyObject,
        worker_kind: WorkerKind,
        #[allow(unused_variables)] policy: Option<String>,
        exposed_as: Option<String>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let name = name.to_string();
        py_debug(py, format!("starting worker '{name}'"))?;

        let address: Address = name.clone().into();
        if let Some(api_sc_listener) = self.node_manager.api_sc_listener() {
            self.ctx()
                .flow_controls()
                .add_consumer(&address, api_sc_listener.flow_control_id());
        }
        let ctx = self.ctx().get_router_context();
        let exposed_workers = self.exposed_workers.clone();
        let started_agents = self.started_agents.clone();
        let started_workers = self.started_workers.clone();
        let is_remote = self.is_remote();

        future_into_py(py, async move {
            let (incoming_ac, outgoing_ac) = (Arc::new(AllowAll), Arc::new(AllowAll));

            match worker_kind {
                WorkerKind::Spawner { key_extractor } => {
                    Spawner::start(
                        &ctx,
                        &name,
                        py_worker_or_factory,
                        key_extractor,
                        started_agents.clone(),
                        incoming_ac,
                        outgoing_ac,
                    )
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Error: {}", e))
                    })?;

                    started_agents
                        .started(name.clone(), is_remote, exposed_as.clone())
                        .await;
                }
                WorkerKind::User | WorkerKind::Internal => {
                    let worker = PyWorker::new(py_worker_or_factory);
                    WorkerBuilder::new(worker)
                        .with_address(address)
                        .with_incoming_access_control_arc(incoming_ac)
                        .with_outgoing_access_control_arc(outgoing_ac)
                        .start_using_router_context(&ctx)
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                                "Error: {}",
                                e
                            ))
                        })?;

                    let worker_type = match worker_kind {
                        WorkerKind::User => WorkerType::UserWorker,
                        WorkerKind::Internal => WorkerType::InternalWorker,
                        _ => unreachable!(),
                    };
                    started_workers.started(name.clone(), worker_type).await;
                }
            };

            if let Some(exposed_as) = exposed_as {
                exposed_workers.expose(name, exposed_as);
            }

            Ok(())
        })
    }

    pub fn policy(&self, policy: Option<String>) -> Result<Expr> {
        Self::policy_static(policy, self.default_policy.clone())
    }

    pub fn policy_static(policy: Option<String>, default_policy: Expr) -> Result<Expr> {
        let policy = if let Some(policy) = policy {
            Some(PolicyExpression::from_str(policy.as_str())?.to_expression())
        } else {
            None
        };

        let policy = policy.unwrap_or(default_policy);

        Ok(policy)
    }

    /// Connect a secure channel route to the destination if a project and a target node are specified.
    /// In that case close the connection when the call has been completed.
    /// Otherwise the route is just used for a local call and we don't need to create a connection.
    pub(crate) async fn with_route<R, Fut, F>(
        &self,
        node_name: Option<String>,
        destination: String,
        fut: F,
    ) -> Result<R>
    where
        R: Send + 'static,
        Fut: Future<Output = Result<R>> + Send,
        F: FnOnce(Route) -> Fut + Send + 'static,
    {
        if let (Some(project), Some(node_name)) = (&self.project, node_name) {
            if let Some(cache) = &self.remote_connection_cache {
                let route = self
                    .get_remote_route_to_node(cache, project.name().to_string(), node_name.clone())
                    .await?;
                let r = fut(route.modify().append(destination).build()).await;
                if r.is_err() {
                    // Maybe the error is because underling connection is not working.  Try to recover from that case
                    self.remove_cached_connection(cache, project.name().to_string(), node_name)
                        .await;
                }
                r
            } else {
                let connection = self
                    .node_manager
                    .make_connection(
                        &MultiAddr::from_str(&format!(
                            "/project/{}/service/forward_to_{}/secure/api/service/{}",
                            project.name(),
                            node_name,
                            destination
                        ))?,
                        self.node_manager.identifier(),
                        None,
                        None,
                    )
                    .await?;
                let route = connection.route()?;

                // FIXME
                // let result = fut(route).await;
                // _ = connection.close(&self.node_manager);
                // result

                fut(route).await
            }
        } else {
            fut(destination.into()).await
        }
    }

    async fn get_remote_route_to_node(
        &self,
        cache: &Arc<RwLock<BTreeMap<(String, String), CachedRemoteRoute>>>,
        project_name: String,
        node_name: String,
    ) -> Result<Route> {
        let destination = MultiAddr::from_str(&format!(
            "/project/{}/service/forward_to_{}/secure/api",
            project_name, node_name
        ))?;
        let key = &(project_name, node_name);
        let mut cache = cache.write().await;
        let now = Instant::now();
        if let Some(cached) = cache.get_mut(key) {
            // Longer duration was observed to work, but I'm letting them at 3 minutes since I'm not sure
            // _why_ they work (unused secure channel/tcp connections get terminated by the relay node, agressively
            // now)
            if now.duration_since(cached.last_used) < Duration::from_secs(180) {
                cached.last_used = now;
                return Ok(cached.route.clone());
            } else {
                // I don't know what happens to the underling tcp and secure channel
                cache.remove(key);
            }
        }
        let connection = self
            .node_manager
            .make_connection(&destination, self.node_manager.identifier(), None, None)
            .await?;
        let route = connection.route()?;
        cache.insert(
            key.to_owned(),
            CachedRemoteRoute {
                last_used: now,
                route: route.clone(),
            },
        );
        Ok(route)
    }

    async fn remove_cached_connection(
        &self,
        cache: &Arc<RwLock<BTreeMap<(String, String), CachedRemoteRoute>>>,
        project_name: String,
        node_name: String,
    ) {
        let key = &(project_name, node_name);
        let mut cache = cache.write().await;
        cache.remove(key);
    }

    fn send_and_receive_impl<'a>(
        &self,
        py: Python<'a>,
        node: Option<String>,
        destination: String,
        message: String,
        #[allow(unused_variables)] policy: Option<String>,
        timeout: Option<u64>,
    ) -> PyResult<Bound<'a, PyAny>> {
        let context = self.ctx().get_router_context();
        let self_clone = self.clone();

        future_into_py(py, async move {
            self_clone
                .with_route(node, destination, move |route| async move {
                    let (incoming_ac, outgoing_ac) = (Arc::new(AllowAll), Arc::new(AllowAll));

                    let options = MessageSendReceiveOptions::new()
                        .with_incoming_access_control(incoming_ac)
                        .with_outgoing_access_control(outgoing_ac);

                    let options = if let Some(timeout) = timeout {
                        options.with_timeout(Duration::from_secs(timeout))
                    } else {
                        options
                    };

                    context
                        .send_and_receive_extended::<String, String>(route, message, options)
                        .await?
                        .into_body()
                })
                .await
                .map_err(py_error)
        })
    }
}
