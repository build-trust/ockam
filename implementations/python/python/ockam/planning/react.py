from typing import Optional, AsyncGenerator, List

from .protocol import Planner, Plan
from ..nodes.message import SystemMessage, ConversationMessage, UserMessage, ConversationRole, Phase
from ..models import Model


class ReActPlan(Plan):
    def __init__(self, steps: list[str]):
        super().__init__()
        self.steps = steps
        self.step_index = 0

    async def next_step(
        self, messages: List[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> AsyncGenerator[ConversationMessage | None, None]:
        if self.step_index >= len(self.steps):
            yield None

        step = self.steps[self.step_index]
        self.step_index += 1

        yield UserMessage(step, Phase.PLANNING)


class ReActPlanner(Planner):
    def __init__(self, model: Model = None):
        if model is None:
            model = Model("deepseek-r1")

        self.model = model

    async def plan(
        self, messages: List[ConversationMessage], contextual_knowledge: Optional[str], stream: bool = False
    ) -> Plan:
        return ReActPlan(await self._plan(messages, contextual_knowledge))

    async def _plan(self, messages: List[ConversationMessage], contextual_knowledge: Optional[str]) -> list[str]:
        if len(messages) == 0:
            return []

        step_messages: List[ConversationMessage] = []
        if contextual_knowledge:
            step_messages.append(
                SystemMessage(f"This information could be useful for proper planning:\n{contextual_knowledge}")
            )

        step_messages.append(
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
5. Each step MUST be separated by a <step> tag.
For example:
```
<step>Step 1: Do something</step>
<step>Step 2: Do something else</step>
...
```

Make sure that the last step reaches the goal of the task.
"""
            )
        )

        if messages[-1].role != ConversationRole.USER:
            raise Exception(f"Role {messages[-1].role} is not supported in the last message, only 'USER' is allowed.")

        step_messages.append(messages[-1])

        plan = await self.model.complete_chat(
            messages=step_messages,
            temperature=0,
        )

        for _ in range(3):
            text = plan.choices[0].message.content
            if "</think>" in text:
                # deepseek uses <think> and </think> tags to indicate the reasoning
                text = text.split("</think>")[1]

            if "<step>" not in text:
                continue

            steps = []
            for step in text.split("<step>")[1:]:
                step = step.replace("</step>", "")
                step = step.strip()
                if len(step) <= 1:
                    continue
                steps.append(step)

            return steps

        # failed to create a plan
        print("Failed to create a plan after 3 attempts", flush=True)
        return [messages[-1].content]
