from ockam import Agent, Model, Node, info

"""
    This example shows how to start an agent locally given a node.
    The agent has:
        - A name.
        - Some instructions
        - A model to use.

    The agent can be sent a message and will respond with a message, using its instructions and its model to do so.
"""


async def main(node):
    agent = await Agent.start(
        node=node,
        name="history-teacher",
        instructions="You are an assistant who is an expert in history.",
        model=Model(name="ollama_chat/llama3.2"),
    )
    identifier = await agent.identifier()
    info(f"the agent identifier is {identifier}")

    reply = await agent.send("Who was Gandhi?")
    info(reply)


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/05.py
