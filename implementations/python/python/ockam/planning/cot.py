from typing import Optional, AsyncGenerator

from ockam.nodes.message import ConversationRole, UserMessage, AssistantMessage, Phase

from .protocol import Planner, Plan
from .utils import complete_think_chat
from ..nodes.message import ConversationMessage, SystemMessage
from ..models import Model


class CotPlan(Plan):
    def __init__(self, model, query: str, stream: bool = False):
        self.model = model
        self.stream = stream
        self.already_planned = False
        self.query = query

    async def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> AsyncGenerator[ConversationMessage, None]:
        if self.already_planned:
            return
        self.already_planned = True
        async for step in self._next_step(messages, contextual_knowledge):
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

        step_messages.extend(
            [
                SystemMessage(
                    """
# Planning Assistant Instructions

You are a specialized planning assistant. Your role is to break down tasks into clear, logical sequences of abstract steps WITHOUT executing those steps yourself.

## Your Responsibilities:
- Analyze the user's task and determine what high-level steps would be required
- Structure the plan in a logical sequence from start to completion
- Keep steps abstract and conceptual - avoid specific implementations or calculations
- Include all necessary steps without skipping important parts of the process
- Consider dependencies between steps
- Focus on WHAT needs to be done, not HOW to do it

## Your Limitations:
- DO NOT perform calculations or executions
- DO NOT write code to implement solutions
- DO NOT resolve the task itself
- DO NOT include specific numerical results

## Format Your Response:
1. Start with a brief overview of your understanding of the task
2. Present your plan as numbered steps
3. Each step should be clearly labeled and explained in 1-2 sentences
4. End with a summary statement

Make sure that the last step reaches the goal of the task.
"""
                ),
                UserMessage(self.query),
                AssistantMessage(
                    f"""<think>Alright, so I need to create a plan to address the task: "{self.query}". """
                ),
            ]
        )

        async for chunk in complete_think_chat(self.model, step_messages, self.stream, Phase.THINKING):
            yield chunk


class CoTPlanner(Planner):
    def __init__(self, model=Model(name="deepseek-r1")):
        self.model = model

    async def plan(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str], stream: bool = False
    ) -> Plan:
        if messages[-1].role != ConversationRole.USER:
            raise Exception(f"Role {messages[-1].role} is not supported in the last message, only 'USER' is allowed.")

        return CotPlan(self.model, messages[-1].content, stream=stream)
