from __future__ import annotations
from typing import Protocol, Optional, Callable, List, runtime_checkable


@runtime_checkable
class MailboxProtocol(Protocol):
    async def send(self, destination: str, message: str, policy: Optional[str] = None) -> None: ...

    async def send_to_remote(self, node: str, destination: str, message: str, policy: Optional[str] = None) -> None: ...

    async def receive(self, policy: Optional[str] = None, timeout: Optional[int] = None) -> str: ...


@runtime_checkable
class ContextProtocol(Protocol):
    async def reply(self, message: str): ...


@runtime_checkable
class WorkerProtocol(Protocol):
    async def handle_message(self, context: ContextProtocol, message: str): ...


@runtime_checkable
class NodeProtocol(Protocol):
    async def send_and_receive(
        self,
        destination: str,
        message: str,
        policy: Optional[str] = None,
        timeout: Optional[int] = None,
    ) -> None: ...

    async def identifier(self) -> str: ...

    async def start_worker(
        self, name: str, worker: WorkerProtocol, policy: Optional[str] = None, exposed_as: Optional[str] = None
    ): ...

    async def stop_worker(self, name: str): ...

    async def list_agents(self): ...

    async def list_workers(self): ...

    async def list_nodes_priv(self): ...

    @property
    def is_remote(self) -> bool: ...

    @property
    def name(self) -> str: ...

    @property
    def local_node(self) -> LocalNodeProtocol: ...


@runtime_checkable
class LocalNodeProtocol(NodeProtocol, Protocol):
    async def send_and_receive_to_remote(
        self,
        node: str,
        destination: str,
        message: str,
        policy: Optional[str] = None,
        timeout: Optional[int] = None,
    ) -> None: ...

    async def send(self, destination: str, message: str, policy: Optional[str] = None) -> MailboxProtocol: ...

    async def call_mcp_tool(self, server_name: str, tool_name: str, tool_args_as_json: Optional[str]) -> str: ...

    async def create_mailbox(self, address: str, policy: Optional[str] = None) -> MailboxProtocol: ...

    async def interrupted(self) -> None: ...

    async def list_tools(self) -> List[dict]: ...

    async def mcp_tool_spec(self, server_name: str, tool_name: str) -> str: ...

    async def mcp_tools(self) -> str: ...

    async def start_spawner(
        self,
        name: str,
        agent_factory: Callable[[], WorkerProtocol],
        key_extractor: Callable[[str], str],
        policy: Optional[str] = None,
        exposed_as: Optional[str] = None,
    ): ...

    async def stop(self) -> None: ...
