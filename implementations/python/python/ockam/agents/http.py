import asyncio
import json
import os
import uvicorn
from dataclasses import asdict, is_dataclass
from enum import Enum
from fastapi import FastAPI, Depends, HTTPException, Request, status
from fastapi.responses import StreamingResponse, FileResponse
from fastapi.security.api_key import APIKeyQuery
from fastapi import Security
from typing import Annotated

from .socket_address import parse_host_and_port
from ..agents import AgentReference
from ..nodes.message import GetConversationsRequest, StreamedConversationSnippet, Phase, Reference, FlowReference
from ..nodes.local import LocalNode
from ..logging.logging import InfoContext, get_logging_config

"""
    This class starts an HTTP server allowing a user to interact with a node and its agents.
"""

DEFAULT_HOST = os.environ.get("DEFAULT_HOST_HTTP", "0.0.0.0")
DEFAULT_PORT = int(os.environ.get("DEFAULT_PORT_HTTP", "8000"))


class HttpServer(InfoContext):
    def __init__(self, listen_address=f"{DEFAULT_HOST}:{DEFAULT_PORT}", app: FastAPI = None, api=None):
        from ..logging.logging import get_logger

        self.logger = get_logger("http")
        self.node = None
        self.host = DEFAULT_HOST
        self.port = DEFAULT_PORT
        self.set_host_and_port(listen_address)
        self.api = api

        if not app:
            self.app = FastAPI(dependencies=[Depends(validate_api_key)])
        else:
            self.app = app
        self.app.middleware("http")(self.inject_node)
        self._setup_routes()

    async def inject_node(self, request: Request, call_next):
        request.state.node = self.node
        return await call_next(request)

    def _setup_routes(self):
        if os.path.exists("index.html"):

            @self.app.get("/")
            async def index():
                return FileResponse("index.html")

        @self.app.get("/agents")
        async def get_agents(node=Depends(self.get_node)):
            with self.info("Retrieving agents", "Retrieved agents"):
                return {"agents": await node.list_agents()}

        @self.app.get("/workers")
        async def get_workers(node=Depends(self.get_node)):
            with self.info("Retrieving workers", "Retrieved workers"):
                return {"workers": await node.list_workers()}

        @self.app.get("/agents/{name}")
        async def get_agent_by_name(name: str, node=Depends(self.get_node)):
            with self.info(f"Retrieving agent with name '{name}'", f"Retrieved agent with name '{name}'"):
                return await find_agent(node, name)

        @self.app.get("/agents/{name}/conversations")
        async def get_conversations_by_agent_name(name: str, node=Depends(self.get_node)):
            with self.info(
                f"Retrieving conversations for agent '{name}'", f"Retrieved conversations for agent '{name}'"
            ):
                return await get_agent_by_name_and_scope_and_conversation_impl(name, None, None, node)

        @self.app.get("/agents/{name}/scopes/{scope}/conversations")
        async def get_conversations_by_agent_name_and_scope(name: str, scope: str, node=Depends(self.get_node)):
            with self.info(
                f"Retrieving conversations for agent '{name}' and scope '{scope}'",
                f"Retrieved conversations for agent '{name}' and scope '{scope}'",
            ):
                return await get_agent_by_name_and_scope_and_conversation_impl(name, scope, None, node)

        @self.app.get("/agents/{name}/scopes/{scope}/conversations/{conversation}")
        async def get_agent_by_name_and_scope_and_conversation(
            name: str, scope: str, conversation: str, node=Depends(self.get_node)
        ):
            with self.info(
                f"Retrieving conversations for agent '{name}', scope '{scope}' and conversation '{conversation}'",
                f"Retrieved conversations for agent '{name}', scope '{scope}' and conversation '{conversation}'",
            ):
                return await get_agent_by_name_and_scope_and_conversation_impl(name, scope, conversation, node)

        async def get_agent_by_name_and_scope_and_conversation_impl(
            name: str, scope: None | str, conversation: None | str, node
        ):
            # Check if the agent exists
            await find_agent(node, name)

            try:
                agent = AgentReference(name, node)
                query = GetConversationsRequest(scope, conversation)
                response = await agent.send_and_receive_request(query)
                return response
            except Exception as e:
                self.logger.error(f"Failed to get the conversations for agent '{name}': {e}")
                raise HTTPException(status_code=500, detail="Failed to get the conversations for agent '{name}'")

        @self.app.post("/agents/{name}")
        async def send_message_to_agent(
            name: str,
            message: Request,
            stream: bool = False,
            content_size: int = 50,
            timeout: int = 60,
            node=Depends(self.get_node),
        ):
            with self.info(f"Sending a message to agent '{name}'", f"Sent a message to agent '{name}'"):
                await find_agent(node, name)
                return await send_message_to_reference(
                    AgentReference(name, node), message, stream, content_size, timeout
                )

        @self.app.post("/flows/{name}")
        async def send_message_to_flow(
            name: str,
            message: Request,
            stream: bool = False,
            content_size: int = 50,
            timeout: int = 60,
            node=Depends(self.get_node),
        ):
            with self.info(f"Sending a message to flow '{name}'", f"Sent a message to flow '{name}'"):
                await find_worker(node, name)
                return await send_message_to_reference(
                    FlowReference(name, node), message, stream, content_size, timeout
                )

        async def find_agent(node, name):
            agents = await node.list_agents()
            agent = next((a for a in agents if a.get("name") == name), None)
            if agent is None:
                raise HTTPException(status_code=404, detail=f"Agent '{name}' not found")
            return agent

        async def find_worker(node, name):
            workers = await node.list_workers()
            worker = next((w for w in workers if w.get("name") == name), None)
            if worker is None:
                raise HTTPException(status_code=404, detail=f"Worker '{name}' not found")
            return worker

        async def find_tool(node, name):
            tools = await node.list_tools()
            tool = next((t for t in tools if t.get("name") == name), None)
            if tool is None:
                raise HTTPException(status_code=404, detail=f"Tool '{name}' not found")
            return tool

        async def send_message_to_reference(
            reference: Reference, message: Request, stream: bool = False, content_size: int = 50, timeout: int = 60
        ):
            try:
                message_json = await message.json()
                if "message" in message_json:
                    msg = message_json["message"]
                else:
                    msg = message_json
                scope = message_json.get("scope", None)
                conversation = message_json.get("conversation", None)

                if stream:

                    async def stream_response():
                        received_snippet = None

                        async for response in reference.send_stream(msg, scope, conversation, timeout=timeout):
                            received = response.snippet
                            # even if we make a streaming request, the response might not be streaming if a downstream
                            # service does not support streaming
                            if type(response) is StreamedConversationSnippet:
                                # if phase is missing or None, set "executing" phase
                                if not hasattr(received, "phase") or received.phase is None:
                                    received.phase = Phase.EXECUTING
                                if not received_snippet:
                                    received_snippet = received
                                else:
                                    yield json.dumps(received_snippet, default=default) + "\n"
                                    received_snippet = received
                                    if response.finished:
                                        break
                            else:
                                yield json.dumps(received, default=default) + "\n"
                                break

                    return StreamingResponse(stream_response(), media_type="application/json")
                else:
                    return await reference.send(msg, scope, conversation, timeout=timeout)
            except Exception as e:
                self.logger.error(f"Failed to send message to '{reference.name}': {e}")
                raise HTTPException(status_code=500, detail="Failed to send message")

        @self.app.get("/tools")
        async def get_tools(node=Depends(self.get_node)):
            with self.info("Get the exposed tools", "Retrieved the exposed tools"):
                return node.list_tools()

        @self.app.get("/tools/{name}")
        async def get_tool_by_name(name: str, node=Depends(self.get_node)):
            with self.info(f"Get tool: {name}", f"Retrieved tool: {name}"):
                return find_tool(node, name)

    def get_node(self):
        return self.node

    async def serve(self):
        self.logger.info(f"Starting http server at {self.host}:{self.port}")
        config = uvicorn.Config(
            self.app, host=self.host, port=self.port, log_config=get_logging_config(), access_log=True
        )
        server = uvicorn.Server(config)
        self.logger.info(f"Started http server at {self.host}:{self.port}")
        await server.serve()

    def set_host_and_port(self, listen_address):
        try:
            host, port = parse_host_and_port(listen_address)
            self.host = host
            self.port = port
        except ValueError:
            raise ValueError(f"Invalid listen_address: {listen_address}")

    async def start(self, node):
        self.node = node
        # mount some additional if provided custom routes
        if self.api:
            self.api.routes(self.node)
            self.app.mount("/", self.api.api)

        asyncio.create_task(self.serve())


# Retrieve the local node from the request state
def get_local_node(request: Request) -> LocalNode:
    return LocalNode(request.state.node)


# LocalNode as a dependency
NodeDep = Annotated[LocalNode, Depends(get_local_node)]


def default(obj):
    if is_dataclass(obj):
        return asdict(obj)
    if isinstance(obj, Enum):
        return obj.value
    raise TypeError(f"Object of type {obj.__class__.__name__} is not JSON serializable")


def validate_api_key(api_key: str = Security(APIKeyQuery(name="api_key", auto_error=False))) -> str:
    expected = os.environ.get("API_KEY")
    if api_key != expected:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid or missing API key header")
    return expected
