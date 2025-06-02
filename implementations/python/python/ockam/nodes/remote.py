from typing import Optional

from .protocol import NodeProtocol
from .manager import RemoteManagerClient
from .protocol import LocalNodeProtocol, WorkerProtocol


class RemoteNode(NodeProtocol):
    node_that_this_object_is_on: LocalNodeProtocol
    name_of_remote_node: str

    def __init__(self, node_that_this_object_is_on: LocalNodeProtocol, name_of_remote_node: str):
        self.node_that_this_object_is_on = node_that_this_object_is_on
        self.name_of_remote_node = name_of_remote_node

    @property
    def is_remote(self) -> bool:
        return True

    @property
    def name(self) -> str:
        return self.name_of_remote_node

    # default timeout of 10 minutes
    async def send_and_receive(self, destination: str, message: str, policy: Optional[str] = None, timeout=600):
        return await self.node_that_this_object_is_on.send_and_receive_to_remote(
            self.name_of_remote_node, destination, message, policy, timeout
        )

    async def identifier(self) -> str:
        client = RemoteManagerClient(self)
        return await client.identifier()

    async def start_worker(self, name: str, worker: WorkerProtocol, policy: Optional[str] = None, exposed_as=None):
        client = RemoteManagerClient(self)
        await client.start_worker(name, worker, policy, exposed_as)

    async def stop_worker(self, name: str):
        client = RemoteManagerClient(self)
        await client.stop_worker(name)

    # TODO: Maybe replace with start_spawner
    async def start_agent(
        self, instructions: str, name: str, model, tools, planner, exposed_as, knowledge, max_knowledge_size
    ):
        client = RemoteManagerClient(self)
        await client.start_agent(instructions, name, model, tools, planner, exposed_as, knowledge, max_knowledge_size)

    async def start_agents(self, instructions, number_of_agents, model, tools, planner, knowledge, max_knowledge_size):
        client = RemoteManagerClient(self)
        return await client.start_agents(
            instructions, number_of_agents, model, tools, planner, knowledge, max_knowledge_size
        )

    async def list_agents(self):
        client = RemoteManagerClient(self)
        return await client.list_agents()

    async def list_workers(self):
        client = RemoteManagerClient(self)
        return await client.list_workers()

    async def list_nodes_priv(self):
        client = RemoteManagerClient(self)
        return await client.list_workers()
