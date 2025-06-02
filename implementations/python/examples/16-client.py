import sys

from ockam import Node, RemoteNode
from ockam.tools import NmapClient


async def main(node):
    remote_node = RemoteNode(node, sys.argv[1])
    client = NmapClient(remote_node)

    manual = await client.get_manual(timeout=5)
    print(f"{manual}\n\n")

    output = await client.run_command("nmap -T4 -F 127.0.0.1", timeout=60)
    print(f"{output}\n\n")


Node.start(main, wait_until_interrupted=False)
