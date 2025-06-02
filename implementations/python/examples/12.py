from ockam import Agent, Node, Repl
from sys import argv

"""
    This example is like example 11 but shows that another model can be used.
"""


async def main(node):
    agent = await Agent.start(
        node=node,
        # model=Model("bedrock/anthropic.claude-3-5-sonnet-20241022-v2:0"),
        instructions="""
            You are Henry, an expert legal assistant.
            You have in-depth knowledge of United States corporate law.
        """,
    )

    await Repl.start(agent, argv[1])


Node.start(main)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/12.py 127.0.0.1:7000
