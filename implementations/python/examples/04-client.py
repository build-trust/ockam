import sys

from ockam import Node, RemoteNode

"""
  This example shows how an enrolled can send messages to a remote node:

  - 04-echo.py starts a node with an Echoer worker
  - Then we can either:
    - Send a message from the local node to the Echoer worker
    - Send a message from a remote node wrapping the local node to the Echoer worker.
      Remote node will be more useful in example 7, where we show that we can send and execute code remotely.
"""


async def main(node):
    reply = await node.send_and_receive(node=sys.argv[1], destination="echoer", message="hello")
    print(reply)

    remote_node = RemoteNode(node, sys.argv[1])
    for i in range(5):
        reply = await remote_node.send_and_receive(destination="echoer", message="hello")
        print(f"{i}> {reply}")


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 CLUSTER=acme NODE=node1 ENROLLMENT_TICKET="$(ockam project ticket --relay node1 --attribute cluster=acme)" uv run examples/04-client.py node2
