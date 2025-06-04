from typing import Optional, List, AsyncGenerator
from .protocol import Planner, Plan
from ..nodes.message import ConversationMessage, SystemMessage, UserMessage
from ..models import Model


class DynamicPlan(Plan):
    MAX_STEPS = 10

    def __init__(self, model):
        self.model = model
        self.step_counter = 0
        self.objective_completed = False

    async def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> AsyncGenerator[ConversationMessage, None]:
        if self.objective_completed:
            return

        self.step_counter += 1
        if self.step_counter > DynamicPlan.MAX_STEPS:
            return

        async for step in await self._next_step(messages, contextual_knowledge):
            if step is None:
                return

            yield step

    async def _next_step(
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

        is_completed_query = step_messages[:]
        is_completed_query.append(
            SystemMessage(
                """Is the objective completed?
Only answer with "yes" or "no". No explanations. Nothing else.
If you are not sure, answer "no".
For example:
```
What color is the sky? The sky is blue.
Is the objective completed? yes
```
"""
            )
        )
        is_completed = await self.model.complete_chat(
            messages=is_completed_query,
            temperature=0,
        )

        text = is_completed.choices[0].message.content
        if "</think>" in text:
            # deepseek uses <think> and </think> tags to indicate the reasoning
            text = text.split("</think>")[1]

        if "yes" in text.lower():
            self.objective_completed = True
            return None

        step_messages.extend(
            [
                SystemMessage("DO NOT make any conclusion."),
                UserMessage("Ok, do NOT answer, only think about the next abstract step to accomplish the task."),
            ]
        )

        next_step = await self.model.complete_chat(
            messages=step_messages,
            temperature=0,
        )

        text = next_step.choices[0].message.content
        if "</think>" in text:
            # deepseek uses <think> and </think> tags to indicate the reasoning
            text = text.split("</think>")[1]

        text = text.strip()
        if len(text) == 0:
            return None

        return text


class DynamicPlanner(Planner):
    def __init__(self, model=Model("deepseek-r1")):
        self.model = model

    async def plan(self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]) -> Plan:
        return DynamicPlan(self.model)
