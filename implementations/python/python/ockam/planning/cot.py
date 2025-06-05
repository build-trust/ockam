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
        async for step in self._plan(messages, contextual_knowledge):
            yield step

    async def _plan(
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

        start_of_thinking_section = False
        end_of_thinking_section = False

        try:
            async for response in self.complete_chat(
                messages=step_messages,
                temperature=0,
                stream=self.stream,
            ):
                choice = response.choices[0]
                if hasattr(choice, "delta"):
                    step_content = choice.delta.content
                else:
                    step_content = choice.message.content
                if step_content is None:
                    continue

                if "<think>" in step_content:
                    start_of_thinking_section = True
                if "</think>" in step_content:
                    # deepseek uses <think> and </think> tags to indicate the reasoning
                    end_of_thinking_section = True
                    step_content = step_content.split("</think>")[1]

                if not self.stream or (not start_of_thinking_section or end_of_thinking_section):
                    # don't emit empty content
                    if step_content.strip() != "":
                        yield AssistantMessage(step_content)
        finally:
            yield STEP_BY_STEP_EXECUTION

    async def complete_chat(self, messages: list[ConversationMessage], temperature: float = 0.0, stream: bool = False):
        response = await self.model.complete_chat(messages=messages, temperature=temperature, stream=stream)
        if stream:
            async for chunk in response:
                yield chunk
        else:
            yield response


class CoTPlanner(Planner):
    def __init__(self, model=Model(name="deepseek-r1")):
        self.model = model

    async def plan(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str], stream: bool = False
    ) -> Plan:
        return CotPlan(self.model, stream=stream)
