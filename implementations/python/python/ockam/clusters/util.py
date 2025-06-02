from ..nodes import RemoteNode


async def list_nodes(node, prefix, filter=None):
    nodes = await node.list_nodes_priv()
    # Strip the relay prefix from the node names
    nodes = [n.removeprefix("forward_to_") for n in nodes]
    # Filter the nodes to only include those in our cluster
    nodes = [n for n in nodes if n.startswith(prefix)]
    if filter is not None:
        nodes = [n for n in nodes if filter in n]
    return [RemoteNode(node, n) for n in nodes]
