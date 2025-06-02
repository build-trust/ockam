from typing import List, Protocol

from typing_extensions import Optional
from ..nodes.message import ConversationMessage


class Plan(Protocol):
    async def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> Optional[List[ConversationMessage]]: ...


class Planner(Protocol):
    async def plan(self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]) -> Plan: ...
