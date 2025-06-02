from ..nodes.node import pick_cluster
from .util import list_nodes


class Cluster:
    @staticmethod
    async def nodes(node, filter=None):
        cluster = pick_cluster()
        return await list_nodes(node, cluster, filter=filter)
