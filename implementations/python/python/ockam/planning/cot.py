from typing import Optional, List

from ockam.nodes.message import UserMessage

from .interface import Planner, Plan
from ..nodes.message import ConversationMessage, SystemMessage, AssistantMessage
from ..models import Model


class CotPlan(Plan):
    def __init__(self, model):
        self.model = model
        self.already_planned = False

    async def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> Optional[List[ConversationMessage]]:
        if self.already_planned:
            return None
        steps = await self._plan(messages, contextual_knowledge)
        self.already_planned = True
        return steps

    async def _plan(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> list[ConversationMessage]:
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
        plan = await self.model.complete_chat(
            messages=step_messages,
            temperature=0,
        )

        text = plan.choices[0].message.content
        if "</think>" in text:
            # deepseek uses <think> and </think> tags to indicate the reasoning
            text = text.split("</think>")[1]

        return [AssistantMessage(text), UserMessage("Execute the plan step by step.")]


class CoTPlanner(Planner):
    def __init__(self, model=Model("deepseek-r1")):
        self.model = model

    async def plan(self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]) -> Plan:
        return CotPlan(self.model)
