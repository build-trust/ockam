from ockam import Agent, Model, Node, Tool
from datetime import datetime, UTC


def current_iso8601_utc_time():
    """
    Returns the current UTC time in ISO 8601 format.
    """
    return datetime.now(UTC).isoformat() + "Z"


async def main(node):
    await Agent.start(
        node=node,
        name="henry",
        instructions="You are Henry, an expert legal assistant",
        model=Model("claude-sonnet-4-v1"),
        tools=[Tool(current_iso8601_utc_time)],
    )


Node.start(main)
