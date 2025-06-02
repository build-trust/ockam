from ockam import Agent, Model, Node

"""
  Second part of example 10.
  This starts a remote agent and only allows communication coming from agents in the same cluster (as specified on the enrollment ticket).
"""


async def main(node):
    await Agent.start(
        node=node,
        name="history-teacher",
        instructions="You are an assistant who is an expert in history.",
        model=Model(name="ollama_chat/llama3.2"),
    )


Node.start(main)

# OCKAM_SQLITE_IN_MEMORY=1 CLUSTER=acme NODE=node2 ENROLLMENT_TICKET="$(ockam project ticket --relay node2 --attribute cluster=acme)" uv run examples/10-server.py
