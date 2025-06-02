from typing import Optional, List

from .interface import Planner, Plan
from ..nodes.message import SystemMessage, ConversationMessage, UserMessage
from ..models import Model


class ReActPlan(Plan):
    def __init__(self, model):
        self.model = model
        self.plan = None
        self.step_index = 0

    async def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> Optional[List[ConversationMessage]]:
        if self.plan is None:
            self.plan = await self._plan(messages, contextual_knowledge)
            if self.plan is None:
                print("Failed to create a plan")
                return messages

        if self.step_index >= len(self.plan):
            return None

        step = self.plan[self.step_index]
        self.step_index += 1

        return [UserMessage(step)]

    async def _plan(self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]) -> list[str]:
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
                UserMessage(
                    """Ok, do NOT answer, only think about multiple abstract steps to accomplish the task.
                Each step MUST be separated by a <step> tag. For example:
                <step>Step 1: Do something</step>
                <step>Step 2: Do something else</step>
                ...
                """
                ),
            ]
        )
        plan = await self.model.complete_chat(
            messages=step_messages,
            temperature=0,
        )

        text = plan.choices[0].message.content
        if "</think>" in text:
            # deepseek uses <think> and </think> tags to indicate the reasoning
            plan = text.split("</think>")[1]

        steps = ["Disregard previous query"]
        for step in plan.split("<step>"):
            step = step.replace("</step>", "")
            step = step.strip()
            if len(step) <= 1:
                continue
            steps.append(step)

        return steps


class ReActPlanner(Planner):
    def __init__(self, model=Model("deepseek-r1")):
        self.model = model

    async def plan(self, _messages: list[ConversationMessage], _contextual_knowledge: Optional[str]) -> Plan:
        return ReActPlan(self.model)
