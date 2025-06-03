import asyncio
import json
import os
import uvicorn

from dataclasses import asdict, is_dataclass
from enum import Enum
from fastapi import FastAPI, Depends, HTTPException, Request
from fastapi.responses import StreamingResponse, FileResponse

from ..agents import AgentReference
from ..nodes.message import GetConversationsRequest, ConversationMessage, AssistantMessage
from ..ockam_in_rust_for_python import info, error


"""
    This class starts an HTTP server allowing a user to interact with a node and its agents.
"""

HOST = "0.0.0.0"
PORT = 8000


class HttpServer:
    def __init__(self, listen_address=f"{HOST}:{PORT}", log_level: str = "debug", api=None):
        self.node = None
        self.host = HOST
        self.port = PORT
        self.set_host_and_port(listen_address)
        self.log_level = log_level
        self.app = FastAPI()
        self.api = api

    def get_node(self):
        return self.node

    def _setup_routes(self):
        if os.path.exists("index.html"):

            @self.app.get("/")
            async def index():
                return FileResponse("index.html")

        @self.app.get("/agents")
        async def get_agents(node=Depends(self.get_node)):
            info("getting the agents")
            return {"agents": await node.list_agents()}

        @self.app.get("/workers")
        async def get_workers(node=Depends(self.get_node)):
            info("getting the workers")
            return {"workers": await node.list_workers()}

        @self.app.get("/agents/{name}")
        async def get_agent_by_name(name: str, node=Depends(self.get_node)):
            info(f"getting agent by name: {name}")
            return await find_agent(node, name)

        @self.app.get("/agents/{name}/conversations")
        async def get_conversations_by_agent_name(name: str, node=Depends(self.get_node)):
            info(f"getting conversations by agent {name}")
            return await get_agent_by_name_and_scope_and_conversation_impl(name, None, None, node)

        @self.app.get("/agents/{name}/scopes/{scope}/conversations")
        async def get_conversations_by_agent_name_and_scope(name: str, scope: str, node=Depends(self.get_node)):
            info(f"getting conversations by agent {name} and scope {scope}")
            return await get_agent_by_name_and_scope_and_conversation_impl(name, scope, None, node)

        @self.app.get("/agents/{name}/scopes/{scope}/conversations/{conversation}")
        async def get_agent_by_name_and_scope_and_conversation(
            name: str, scope: str, conversation: str, node=Depends(self.get_node)
        ):
            info(f"getting conversations by agent {name}, scope {scope} and conversation {conversation}")
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
                error(f"Failed to get the conversations for agent '{name}': {e}")
                raise HTTPException(status_code=500, detail="Failed to get the conversations for agent '{name}'")

        @self.app.post("/agents/{name}")
        async def send_message_to_agent(name: str, message: Request, stream: bool = False, content_size: int = 50, node=Depends(self.get_node)):
            info(f"Sending a message to agent '{name}'")
            await find_agent(node, name)

            try:
                agent = AgentReference(name, node)
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

                        async for response in agent.send_stream(msg, scope, conversation):
                            received = response.snippet
                            if not received_snippet:
                                received_snippet = received
                            else:
                                received_snippet.messages += received.messages
                            total_size = sum(len(m.content) for m in received_snippet.messages)
                            if total_size > content_size or response.finished:
                                received_snippet = received_snippet.compact_assistant_messages()
                                yield json.dumps(received_snippet, default=default) + "\n"
                                received_snippet = None
                            if response.finished:
                                break

                    return StreamingResponse(stream_response(), media_type="application/json")
                else:
                    return await agent.send(msg, scope, conversation)
            except Exception as e:
                error(f"Failed to send message to agent '{name}': {e}")
                raise HTTPException(status_code=500, detail="Failed to send message")

        @self.app.get("/tools")
        async def get_tools(node=Depends(self.get_node)):
            info("getting the exposed tools")
            return node.list_tools()

        @self.app.get("/tools/{name}")
        async def get_tool_by_name(name: str, node=Depends(self.get_node)):
            info(f"getting tool by name: {name}")
            return find_tool(node, name)

        # mount the custom routes
        if self.api:
            self.api.routes(self.node)
            self.app.mount("/", self.api.api)

        async def find_agent(node, name):
            agents = await node.list_agents()
            agent = next((a for a in agents if a.get("name") == name), None)
            if agent is None:
                raise HTTPException(status_code=404, detail=f"Agent '{name}' not found")
            return agent

        async def find_tool(node, name):
            tools = await node.list_tools()
            tool = next((t for t in tools if t.get("name") == name), None)
            if tool is None:
                raise HTTPException(status_code=404, detail=f"Tool '{name}' not found")
            return tool

    async def serve(self):
        config = uvicorn.Config(self.app, host=self.host, port=self.port, log_level=self.log_level)
        server = uvicorn.Server(config)
        await server.serve()

    def set_host_and_port(self, listen_address):
        try:
            if isinstance(listen_address, int):
                self.port = listen_address
            elif isinstance(listen_address, str) and ":" in listen_address:
                host, port_str = listen_address.split(":", 1)
                self.host = host
                self.port = int(port_str)
        except ValueError:
            raise ValueError(f"Invalid listen_address: {listen_address}")

    async def start(self, node):
        self.node = node
        self._setup_routes()
        asyncio.create_task(self.serve())


def default(obj):
    if is_dataclass(obj):
        return asdict(obj)
    if isinstance(obj, Enum):
        return obj.value
    raise TypeError(f"Object of type {obj.__class__.__name__} is not JSON serializable")


