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
        instructions="You are a legal expert.",
        model=Model("claude-3-5-sonnet-v1"),
        tools=[
            McpTool("firecrawl", "firecrawl_scrape"),
            McpTool("firecrawl", "firecrawl_search"),
            Tool(current_iso8601_utc_time),
        ],
    )


Node.start(
    main,
    mcp_clients=[
        McpClient(name="firecrawl", address="http://localhost:8001/sse"),
    ],
)
