from typing import Protocol, Optional


class InvokableTool(Protocol):
    async def spec(self) -> dict: ...

    async def invoke(self, json_argument: Optional[str]) -> str: ...
