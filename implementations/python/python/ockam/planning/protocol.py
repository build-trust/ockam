from typing import Protocol, AsyncGenerator

from typing_extensions import Optional
from ..nodes.message import ConversationMessage


class Plan(Protocol):
    def __init__(self):
        self.stream = None
        self.model = None

    def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> AsyncGenerator[ConversationMessage, None]: ...


class Planner(Protocol):
    async def plan(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str], stream: bool = False
    ) -> Plan: ...
