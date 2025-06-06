from datetime import datetime
from ockam import Agent, Model, Node, McpTool, McpClient, Tool


def current_iso8601_utc_time():
    """
    Returns the current UTC time in ISO 8601 format.
    """
    return datetime.utcnow().isoformat() + "Z"


async def main(node):
    await Agent.start(
        node=node,
        name="henry",
        instructions="""
            You are Henry, an expert legal assistant.

            When you are given a question, decide if your response can be better
            with a web search. If so, use the web search tool to improve your response.
        """,
        model=Model("claude-3-7-sonnet-v1"),
        tools=[
            McpTool("brave_search", "brave_web_search"),
            Tool(current_iso8601_utc_time),
        ],
    )


Node.start(
    main,
    mcp_clients=[
        McpClient(name="brave_search", address="http://localhost:8001/sse"),
    ],
)
