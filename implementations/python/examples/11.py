from ockam import Agent, Node, Repl, Model

"""
  This example shows how to start a repl to interact with an agent in the command line.
  Queries can be received until the user types "quit" or CTRL-C.
"""


async def main(node):
    agent = await Agent.start(
        node=node,
        model=Model(name="llama3.2"),
        instructions="""
            You are Henry, an expert legal assistant.
            You have in-depth knowledge of United States corporate law.
        """,
    )

    await Repl.start(agent, stream=True)


Node.start(main)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/11.py
