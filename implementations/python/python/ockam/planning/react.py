from logging import debug

from typing import Optional, List, AsyncGenerator

from .protocol import Planner, Plan
from ..nodes.message import SystemMessage, ConversationMessage, UserMessage, Error, ConversationRole
from ..models import Model


class ReActPlan(Plan):
    def __init__(self, model, stream: bool = False):
        self.model = model
        self.stream = stream
        self.plan_steps: Optional[List[ConversationMessage]] = None

    async def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> AsyncGenerator[ConversationMessage, None]:
        if self.plan_steps is None:
            self.plan_steps = []
            async for step in self._next_step(messages, contextual_knowledge):
                self.plan_steps.append(step)
                yield step
        else:
            for step in self.plan_steps:
                yield step

        return

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
        async for response in await self.model.complete_chat(
            messages=step_messages,
            temperature=0,
            stream=self.stream,
        ):
            step_content = response.message.content
            if step_content is None:
                return
            if "</think>" in step_content:
                # deepseek uses <think> and </think> tags to indicate the reasoning
                yield step_content.replace("<think>", "").replace("</think>", "")
            else:
                yield step_content


class ReActPlanner(Planner):
    def __init__(self, model=Model("deepseek-r1")):
        self.model = model

    async def plan(
        self, _messages: list[ConversationMessage], _contextual_knowledge: Optional[str], stream: bool = False
    ) -> Plan:
        return ReActPlan(self.model, stream=stream)
