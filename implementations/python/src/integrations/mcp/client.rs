use crate::errors::ockam_error;
use crate::nodes::PyNode;

use ockam::compat::sync::Arc;
use ockam::{Address, Context, MessageSendReceiveOptions, Routed, Worker, route};
use rmcp::model::{
    CallToolRequestParam, CallToolResult, ClientCapabilities, ClientInfo, Implementation,
    InitializeRequestParam, ListToolsResult,
};
use rmcp::serde_json::Value;
use rmcp::service::RunningService;
use rmcp::transport::SseTransport;
use rmcp::{RoleClient, ServiceExt, serde_json};

use pyo3::prelude::*;

#[pyclass(name = "McpClient")]
pub struct PyMcpClient {
    #[pyo3(get)]
    name: String,

    #[pyo3(get)]
    address: String,
}

#[pymethods]
impl PyMcpClient {
    #[new]
    fn new(name: String, address: String) -> Self {
        Self { name, address }
    }
}

const WORKER_ADDRESS: &str = "mcp_clients_worker";

#[derive(serde::Serialize, serde::Deserialize, Debug)]
enum ClientsWorkerRequest {
    ListTools,
    CallTool {
        server: String,
        tool: String,
        args: Option<String>,
    },
    GetToolSpec {
        server: String,
        tool: String,
    },
}

struct ClientsWorker {
    clients: Vec<Client>,
}

#[ockam::worker]
impl Worker for ClientsWorker {
    type Message = String;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> ockam::Result<()> {
        let return_route = message.return_route().clone();
        let body = message.into_body()?;
        let request = serde_json::from_str::<ClientsWorkerRequest>(&body).map_err(ockam_error)?;

        match request {
            ClientsWorkerRequest::ListTools => {
                let mut all_tools = Vec::new();
                for client in self.clients.iter() {
                    let tools = client.list_tools().await?;
                    all_tools.extend(tools.tools);
                }

                let r = serde_json::to_string_pretty(&all_tools).map_err(ockam_error)?;
                ctx.send(return_route, r).await?;
            }

            ClientsWorkerRequest::CallTool { server, tool, args } => {
                for client in self.clients.iter() {
                    if client.name == server {
                        let tools = client.list_tools().await?;
                        for t in tools.tools {
                            if t.name == tool {
                                let res = client.call_tool(tool, args).await?;
                                let r = serde_json::to_string_pretty(&res).map_err(ockam_error)?;
                                ctx.send(return_route, r).await?;
                                return Ok(());
                            }
                        }
                    }
                }

                let e = format!("Error: Tool not found {}/{}", server, tool);
                let error_message = serde_json::to_string(&e).map_err(ockam_error)?;
                ctx.send(return_route, error_message).await?;
            }

            ClientsWorkerRequest::GetToolSpec { server, tool } => {
                for client in self.clients.iter() {
                    if client.name == server {
                        let tools = client.list_tools().await?;
                        for t in tools.tools {
                            if t.name == tool {
                                let r = serde_json::to_string_pretty(&t).map_err(ockam_error)?;
                                ctx.send(return_route, r).await?;
                                return Ok(());
                            }
                        }
                    }
                }

                let e = format!("Error: Tool not found {}/{}", server, tool);
                let error_message = serde_json::to_string(&e).map_err(ockam_error)?;
                ctx.send(return_route, error_message).await?;
            }
        }

        Ok(())
    }
}

impl ClientsWorker {
    async fn new(servers: Vec<(String, String)>) -> Result<Self, ockam::Error> {
        let mut clients = Vec::with_capacity(servers.len());

        for (name, address) in servers {
            let transport = SseTransport::start(address.as_str())
                .await
                .map_err(ockam_error)?;

            let connection = Client::info().serve(transport).await.map_err(ockam_error)?;
            clients.push(Client {
                name,
                address,
                connection: Arc::new(connection),
            });
        }

        Ok(Self { clients })
    }
}

struct Client {
    name: String,

    #[allow(dead_code)]
    address: String,

    connection: Arc<RunningService<RoleClient, InitializeRequestParam>>,
}

impl Client {
    async fn call_tool(&self, name: String, args: Option<String>) -> ockam::Result<CallToolResult> {
        let p = CallToolRequestParam {
            name: name.into(),
            arguments: from_json(args).await?,
        };

        let r = self.connection.call_tool(p).await.map_err(ockam_error)?;
        Ok(r)
    }

    async fn list_tools(&self) -> ockam::Result<ListToolsResult> {
        self.connection
            .list_tools(Default::default())
            .await
            .map_err(ockam_error)
    }

    fn info() -> ClientInfo {
        ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "Ockam MCP Client".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }
}

async fn from_json(json: Option<String>) -> ockam::Result<Option<serde_json::Map<String, Value>>> {
    if let Some(j) = json {
        serde_json::from_str::<serde_json::Map<String, Value>>(&j)
            .map_err(ockam_error)
            .map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) async fn start_clients(
    node: &PyNode,
    servers: Vec<(String, String)>,
) -> miette::Result<()> {
    if servers.is_empty() {
        return Err(miette::miette!("addresses vector is empty"));
    }

    let clients_worker = ClientsWorker::new(servers).await?;
    node.node_manager()
        .ctx()
        .start_worker(WORKER_ADDRESS, clients_worker)?;

    Ok(())
}

pub(crate) async fn tool_spec(
    node: &PyNode,
    server: String,
    tool: String,
) -> miette::Result<String> {
    to_worker(node, &ClientsWorkerRequest::GetToolSpec { server, tool }).await
}

pub(crate) async fn call_tool(
    node: &PyNode,
    server: String,
    tool: String,
    args: Option<String>,
) -> miette::Result<String> {
    to_worker(node, &ClientsWorkerRequest::CallTool { server, tool, args }).await
}

pub(crate) async fn list_tools(node: &PyNode) -> miette::Result<String> {
    to_worker(node, &ClientsWorkerRequest::ListTools).await
}

async fn to_worker(node: &PyNode, message: &ClientsWorkerRequest) -> miette::Result<String> {
    let json = serde_json::to_string(message)
        .map_err(|e| miette::miette!(format!("Error in serializing to json: {}", e)))?;

    // TODO: Add timeout
    let options = MessageSendReceiveOptions::new();

    let r = node
        .node_manager()
        .ctx()
        .send_and_receive_extended::<String, String>(
            route![Address::from_string(WORKER_ADDRESS)],
            json,
            options,
        )
        .await
        .map_err(|e| miette::miette!(format!("Error during send_and_receive send: {}", e)))?
        .into_body()
        .map_err(|e| miette::miette!(format!("Error during send_and_receive decode: {}", e)))?;

    Ok(r)
}
