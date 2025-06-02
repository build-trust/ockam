import json
import re
from typing import Optional

from .interface import InvokableTool


class McpTool(InvokableTool):
    def __init__(self, server_name, tool_name):
        self.server_name = server_name
        self.tool_name = tool_name
        self.name = f"{server_name}_{tool_name}"
        self.node = None

        if re.match(r"^[a-z0-9_-]+$", self.name) is None:
            raise ValueError("Server and tool names may only contain [a-z0-9_-] characters")

    async def spec(self):
        mcp_spec = await self.node.mcp_tool_spec(self.server_name, self.tool_name)
        if "not found" in mcp_spec:
            raise ValueError(f"Tool '{self.tool_name}' not found on server '{self.server_name}'")

        mcp_spec = json.loads(mcp_spec)
        spec = {
            "type": "function",
            "function": {
                "name": self.name,
                "description": mcp_spec["description"],
                "parameters": mcp_spec["inputSchema"],
                "strict": False,
            },
        }

        return spec

    async def invoke(self, json_argument: Optional[str]) -> str:
        return await self.node.call_mcp_tool(self.server_name, self.tool_name, json_argument)
