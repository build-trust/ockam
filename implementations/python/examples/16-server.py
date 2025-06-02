from ockam import Node
from ockam.tools import NmapWorker


async def main(node):
    await NmapWorker.start(node)


Node.start(main)
