import sys

from ockam import AgentReference, Node, RemoteNode

"""
    This example shows how to reference an agent started remotely (see 10-server.py, where the agent is created
    and started) and send it messages.

    Note the use of the `allow` attribute to specify that we only want to communicate with an agent on the same cluster as us.
"""


async def main(node):
    remote_node = RemoteNode(node, sys.argv[1])
    agent = AgentReference("history-teacher", remote_node)
    reply = await agent.send("Who was Gandhi?")
    print(reply)


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 CLUSTER=acme NODE=node1 ENROLLMENT_TICKET="$(ockam project ticket --relay node1 --attribute cluster=acme)" uv run examples/10-client.py node2
