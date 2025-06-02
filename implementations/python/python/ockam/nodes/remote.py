from .interface import NodeInterface
from .manager import RemoteManagerClient


class RemoteNode(NodeInterface):
    def __init__(self, node_that_this_object_is_on, name_of_remote_node):
        self.node_that_this_object_is_on = node_that_this_object_is_on
        self.name_of_remote_node = name_of_remote_node

    @property
    def is_remote(self) -> bool:
        return True

    @property
    def name(self) -> str:
        return self.name_of_remote_node

    # default timeout of 10 minutes
    async def send_and_receive(self, destination, message, policy=None, timeout=600):
        return await self.node_that_this_object_is_on.send_and_receive(
            node=self.name_of_remote_node, destination=destination, message=message, policy=policy, timeout=timeout
        )

    async def start_worker(self, name, worker, policy=None):
        client = RemoteManagerClient(self)
        await client.start_worker(name, worker, policy)

    async def start_agent(self, instructions, name, model, tools, planner, exposed_as, knowledge, max_knowledge_size):
        client = RemoteManagerClient(self)
        await client.start_agent(instructions, name, model, tools, planner, exposed_as, knowledge, max_knowledge_size)

    async def start_agents(self, instructions, number_of_agents, model, tools, planner, knowledge, max_knowledge_size):
        client = RemoteManagerClient(self)
        names = await client.start_agents(
            instructions, number_of_agents, model, tools, planner, knowledge, max_knowledge_size
        )

        return names

    async def stop_worker(self, name):
        client = RemoteManagerClient(self)
        await client.stop_worker(name)

    async def list_agents(self):
        client = RemoteManagerClient(self)
        return await client.list_agents()

    async def list_workers(self):
        client = RemoteManagerClient(self)
        return await client.list_workers()
