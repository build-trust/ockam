from typing import Optional, AsyncGenerator

from ockam.nodes.message import UserMessage

from .protocol import Planner, Plan, STEP_BY_STEP_EXECUTION
from ..nodes.message import ConversationMessage, SystemMessage, AssistantMessage
from ..models import Model


class CotPlan(Plan):
    def __init__(self, model, stream: bool = False):
        self.model = model
        self.stream = stream
        self.already_planned = False

    async def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> AsyncGenerator[ConversationMessage, None]:
        if self.already_planned:
            return
        self.already_planned = True
        async for step in self._next_step(messages, contextual_knowledge):
            yield step

    def _next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> AsyncGenerator[ConversationMessage, None]:
        step_messages: list[ConversationMessage] = []
        if contextual_knowledge:
            step_messages.append(
                SystemMessage(f"This information could be useful for proper planning:\n{contextual_knowledge}")
            )

        for message in messages:
            if not isinstance(message, SystemMessage):
                step_messages.append(message)

        step_messages.extend(
            [
                SystemMessage("DO NOT make any conclusion."),
                UserMessage("Ok, do NOT answer, only think about multiple abstract steps to accomplish the task."),
            ]
        )

        return self.process_step_messages(step_messages)


class CoTPlanner(Planner):
    def __init__(self, model=Model(name="deepseek-r1")):
        self.model = model

    async def plan(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str], stream: bool = False
    ) -> Plan:
        return CotPlan(self.model, stream=stream)
