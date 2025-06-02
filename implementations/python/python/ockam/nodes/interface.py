from typing import Protocol, Optional, Callable, List


class Mailbox(Protocol):
    async def send(
        self, destination: str, message: str, node: Optional[str] = None, policy: Optional[str] = None
    ) -> None: ...

    async def receive(self, policy: Optional[str] = None, timeout: Optional[int] = None) -> str: ...


class Context(Protocol):
    async def reply(self, message: str): ...


class Worker(Protocol):
    async def handle_message(self, context: Context, message: str): ...


class NodeInterface(Protocol):
    async def send_and_receive(
        self,
        destination: str,
        message: str,
        policy: Optional[str] = None,
        timeout: Optional[int] = None,
    ) -> None: ...

    async def start_worker(
        self, name: str, worker: Worker, policy: Optional[str] = None, exposed_as: Optional[str] = None
    ): ...

    async def stop_worker(self, name: str): ...

    async def list_agents(self): ...

    @property
    def is_remote(self) -> bool: ...

    @property
    def name(self) -> str: ...


class LocalNode(NodeInterface, Protocol):
    async def send_and_receive(
        self,
        destination: str,
        message: str,
        policy: Optional[str] = None,
        timeout: Optional[int] = None,
        node: Optional[str] = None,
    ) -> None: ...

    async def call_mcp_tool(self, server_name: str, tool_name: str, tool_args_as_json: Optional[str]) -> str: ...

    async def create_mailbox(self, address: str, policy: Optional[str] = None) -> Mailbox: ...

    async def identifier(self) -> str: ...

    async def interrupted(self) -> None: ...

    async def list_agents(self) -> List[dict]: ...

    async def list_tools(self) -> List[dict]: ...

    async def mcp_tool_spec(self, server_name: str, tool_name: str) -> str: ...

    async def mcp_tools(self) -> str: ...

    async def start_spawner(
        self,
        name: str,
        agent_factory: Callable[[], Worker],
        key_extractor: Callable[[str], str],
        policy: Optional[str] = None,
        exposed_as: Optional[str] = None,
    ): ...

    async def stop(self) -> None: ...
