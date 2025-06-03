use crate::nodes::PyNode;

use ockam::{Address, MessageSendReceiveOptions, route};

use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, ErrorCode, Implementation,
    InitializeRequestParam, InitializeResult, JsonObject, ListToolsResult, PaginatedRequestParam,
    ProtocolVersion, Tool,
};
use rmcp::serde_json::{Value, json};
use rmcp::service::RequestContext;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use rmcp::{Error, Peer, RoleServer, ServerHandler};

use std::net::AddrParseError;
use std::sync::{Arc, Mutex, OnceLock};
use tokio_util::sync::CancellationToken;

use pyo3::prelude::*;

#[pyclass(name = "McpServer")]
pub struct PyMcpServer {
    #[pyo3(get)]
    listen_address: String,
}

#[pymethods]
impl PyMcpServer {
    #[new]
    fn new(listen_address: String) -> Self {
        Self { listen_address }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ExposedWorkers {
    workers: Arc<Mutex<Vec<Tool>>>,
}

static TOOL_SCHEMA: OnceLock<Arc<JsonObject>> = OnceLock::new();

pub fn tool_schema() -> Arc<JsonObject> {
    TOOL_SCHEMA
        .get_or_init(|| {
            let schema = json!({
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "An explicit prompt to the tool, similar to an AI chat.",
                        "examples": [
                            "What is the weather like today?",
                            "Tell me a joke.",
                            "How do I make a cup of coffee?",
                        ],
                    },
                },
                "required": ["prompt"],
            });
            if let Value::Object(object) = schema {
                Arc::new(object)
            } else {
                panic!("Expected a JSON object")
            }
        })
        .clone()
}

impl ExposedWorkers {
    pub(crate) fn is_present(&self, name: &str) -> bool {
        let guard = self.workers.lock().unwrap();
        guard.iter().any(|t| t.name == *name)
    }
}

impl ExposedWorkers {
    pub(crate) fn expose(&self, name: String, description: String) {
        let tool = Tool::new(name, description, tool_schema());
        let mut guard = self.workers.lock().unwrap();
        guard.push(tool)
    }

    fn tools(&self) -> Vec<Tool> {
        self.workers.lock().unwrap().clone()
    }

    pub fn tools_as_json(&self) -> Value {
        let mut tools = vec![];
        for tool in self.tools() {
            tools.push(Self::tool_as_json(tool));
        }
        Value::Array(tools)
    }

    /// Get the tool as a JSON object
    fn tool_as_json(tool: Tool) -> Value {
        let mut json_object = JsonObject::new();
        json_object.insert("name".to_string(), Value::String(tool.name.to_string()));
        json_object.insert(
            "description".to_string(),
            Value::String(tool.description.to_string()),
        );
        json_object.insert(
            "input_schema".to_string(),
            Value::from((*tool.input_schema).clone()),
        );
        Value::Object(json_object)
    }
}

#[derive(Clone)]
struct SseEntrypoint {
    node: PyNode,
    active_sessions: Arc<Mutex<Vec<Peer<RoleServer>>>>,
    exposed_workers: ExposedWorkers,
}

impl SseEntrypoint {
    fn new(node: PyNode, exposed_workers: ExposedWorkers) -> Self {
        Self {
            node,
            exposed_workers,
            active_sessions: Default::default(),
        }
    }
}

impl ServerHandler for SseEntrypoint {
    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, Error> {
        {
            // store the peer in the active sessions list so we can send notifications later
            let mut guard = self.active_sessions.lock().unwrap();
            guard.push(context.peer.clone());
        }
        Ok(self.get_info())
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, Error> {
        if !self.exposed_workers.is_present(&request.name) {
            return Err(Error::new(
                ErrorCode::INVALID_PARAMS,
                format!("Tool {} not found", request.name),
                None,
            ));
        }

        let prompt = if let Some(mut arguments) = request.arguments {
            if let Some(prompt) = arguments.remove("prompt") {
                if let Value::String(prompt) = prompt {
                    prompt
                } else {
                    prompt.to_string()
                }
            } else {
                return Err(Error::new(
                    ErrorCode::INVALID_PARAMS,
                    "Missing 'message' argument".to_string(),
                    None,
                ));
            }
        } else {
            return Err(Error::new(
                ErrorCode::INVALID_PARAMS,
                "Tool arguments are required".to_string(),
                None,
            ));
        };

        let message = json!({
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "type": "conversation_snippet",
        })
        .to_string();

        let address = Address::from_string(request.name);

        // TODO: Add timeout
        let options = MessageSendReceiveOptions::new();

        let response = self
            .node
            .node_manager()
            .ctx()
            .send_and_receive_extended::<String, String>(route![address], message, options)
            .await
            .map_err(|_err| {
                Error::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Failed to send message".to_string(),
                    None,
                )
            })?
            .into_body()
            .map_err(|_err| {
                Error::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Failed to decode message".to_string(),
                    None,
                )
            })?;

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            next_cursor: None,
            tools: self.exposed_workers.tools(),
        })
    }

    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "Ockam".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: None,
        }
    }
}

pub(crate) async fn start_server(
    node: PyNode,
    exposed_workers: ExposedWorkers,
    listen_address: &str,
) -> miette::Result<()> {
    let server = SseServer::serve_with_config(SseServerConfig {
        bind: listen_address
            .parse()
            .map_err(|e: AddrParseError| miette::miette!(e))?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
    })
    .await
    .map_err(|e| miette::miette!(e))?;

    let service = SseEntrypoint::new(node, exposed_workers);
    server.with_service(move || service.clone());

    Ok(())
}
