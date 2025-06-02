from os import getenv
from ..nodes.node import pick_cluster
from .util import list_nodes


class Zone:
    @staticmethod
    async def nodes(node, filter=None):
        cluster = pick_cluster()
        zone = getenv("ZONE")
        return await list_nodes(node, f"{cluster}-{zone}", filter=filter)
