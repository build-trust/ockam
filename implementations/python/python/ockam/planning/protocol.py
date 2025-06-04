from typing import List, Protocol, AsyncGenerator

from typing_extensions import Optional
from ..nodes.message import ConversationMessage


class Plan(Protocol):
    async def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> AsyncGenerator[ConversationMessage, None]: ...


class Planner(Protocol):
    async def plan(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str], stream: bool = False
    ) -> Plan: ...
