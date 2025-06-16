from typing import Optional, Callable, List

from .protocol import MailboxProtocol
from ..ockam_in_rust_for_python import Node, Mailbox

from .protocol import WorkerProtocol, LocalNodeProtocol


class LocalNode(LocalNodeProtocol):
    def __init__(self, rust_node: Node):
        self.rust_node = rust_node

    async def send_and_receive(
        self,
        destination: str,
        message: str,
        policy: Optional[str] = None,
        timeout: Optional[int] = None,
    ) -> None:
        return await self.rust_node.send_and_receive(destination, message, policy, timeout)

    async def send(self, destination: str, message: str, policy: Optional[str] = None) -> MailboxProtocol:
        import secrets

        address = secrets.token_hex(12)
        mailbox = await self.create_mailbox(address, policy)

        await mailbox.send(destination, message)

        return mailbox

    async def send_to_remote(
        self, node: str, destination: str, message: str, policy: Optional[str] = None
    ) -> MailboxProtocol:
        import secrets

        address = secrets.token_hex(12)
        mailbox = await self.create_mailbox(address, policy)

        await mailbox.send_to_remote(node, destination, message)

        return mailbox

    async def identifier(self) -> str:
        return self.rust_node.identifier()

    async def start_worker(
        self, name: str, worker: WorkerProtocol, policy: Optional[str] = None, exposed_as: Optional[str] = None
    ):
        from ..agents import validate_name

        validate_name(name)
        return await self.rust_node.start_worker(name, worker, policy, exposed_as)

    async def start_internal_worker(
        self, name: str, worker: WorkerProtocol, policy: Optional[str] = None, exposed_as: Optional[str] = None
    ):
        return await self.rust_node.start_internal_worker(name, worker, policy, exposed_as)

    async def stop_worker(self, name: str):
        return await self.rust_node.stop_worker(name)

    async def list_agents(self):
        return await self.rust_node.list_agents()

    async def list_workers(self):
        return await self.rust_node.list_workers()

    async def list_nodes_priv(self):
        return await self.rust_node.list_nodes_priv()

    @property
    def is_remote(self) -> bool:
        return False

    @property
    def name(self) -> str:
        return self.rust_node.name

    async def send_and_receive_to_remote(
        self,
        node: str,
        destination: str,
        message: str,
        policy: Optional[str] = None,
        timeout: Optional[int] = None,
    ) -> None:
        return await self.rust_node.send_and_receive_to_remote(node, destination, message, policy, timeout)

    async def call_mcp_tool(self, server_name: str, tool_name: str, tool_args_as_json: Optional[str]) -> str:
        return await self.rust_node.call_mcp_tool(server_name, tool_name, tool_args_as_json)

    async def create_mailbox(self, address: str, policy: Optional[str] = None) -> Mailbox:
        return await self.rust_node.create_mailbox(address, policy)

    async def interrupted(self) -> None:
        return await self.rust_node.interrupted()

    async def list_tools(self) -> List[dict]:
        return await self.rust_node.list_tools()

    async def mcp_tool_spec(self, server_name: str, tool_name: str) -> str:
        return await self.rust_node.mcp_tool_spec(server_name, tool_name)

    async def mcp_tools(self) -> str:
        return await self.rust_node.mcp_tools()

    async def start_spawner(
        self,
        name: str,
        agent_factory: Callable[[], WorkerProtocol],
        key_extractor: Callable[[str], str],
        policy: Optional[str] = None,
        exposed_as: Optional[str] = None,
    ):
        return await self.rust_node.start_spawner(name, agent_factory, key_extractor, policy, exposed_as)

    async def stop(self) -> None:
        return await self.rust_node.stop()
