from typing import List, Protocol, AsyncGenerator

from typing_extensions import Optional
from ..nodes.message import ConversationMessage, UserMessage, AssistantMessage

STEP_BY_STEP_EXECUTION = UserMessage("Execute the plan step by step.")


class Plan(Protocol):
    def __init__(self):
        self.stream = None
        self.model = None

    async def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> AsyncGenerator[ConversationMessage, None]: ...

    async def process_step_messages(
        self, step_messages: List[ConversationMessage]
    ) -> AsyncGenerator[ConversationMessage, None]:
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


class Planner(Protocol):
    async def plan(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str], stream: bool = False
    ) -> Plan: ...
