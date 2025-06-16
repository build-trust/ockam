from typing import Optional, AsyncGenerator
from .protocol import Planner, Plan
from .utils import complete_think_chat
from ..nodes.message import ConversationMessage, SystemMessage, UserMessage, AssistantMessage, ConversationRole, Phase
from ..models import Model


class DynamicPlan(Plan):
    MAX_STEPS = 10

    def __init__(self, model, initial_query: str, stream: bool = False):
        self.model = model
        self.stream = stream
        self.step_counter = 0
        self.initial_query = initial_query
        self.objective_completed = False

    async def next_step(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str]
    ) -> AsyncGenerator[ConversationMessage, None]:
        if self.objective_completed:
            return

        self.step_counter += 1
        if self.step_counter > DynamicPlan.MAX_STEPS:
            return

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

        is_completed_query = step_messages[:]
        is_completed_query.extend(
            [
                SystemMessage(
                    """You must figure out whether the provided task has been completed or not.
Only answer with "yes" or "no". No explanations. Nothing else.
For example:
```
The sky is blue.
<task>What color is the sky? The sky is blue.</task>
<think>Alright, so I need to figure out whether the task "What color is the sky?" has been completed or not.
The task is asking for the color of the sky, and the provided information states that "The sky is blue." This means the task has been completed successfully.</think>
<completed>yes</completed>
```

```
The sky is blue.
<task>Why is water salty?</task>
<think>Alright, so I need to determine whether the task "Why is water salty?" has been completed or not.
The task is asking for the reason why water is salty, but the provided information does not address this question at all. Therefore, the task has not been completed.</think>
<completed>no</completed>
```

```
<task>What color is the sky? The sky is blue.</task>
<think>Alright, so I need to figure out whether the task "What color is the sky?" has been completed or not.
Since there is no response provided, I must assume the task has not been completed.</think>
<completed>no</completed>
```

```
The capital of France is Paris, so no, Madrid is not the capital of France.
<task>Is Madrid the capital of France?</task>
<think>Alright, so I need to understand whether the task "Is Madrid the capital of France?" has been completed or not.
The task is asking if Madrid is the capital of France, and the provided information states that "no, Madrid is not the capital of France" because "The capital of France is Paris." This means the task has been completed successfully.</think>
<completed>yes</completed>
```

```
(3*5) + 2 = ?
<task>Compute the result of (3*5) + 2?</task>
<think>Alright, so I need to evaluate whether the task "Compute the result of (3*5) + 2?" has been completed or not.
The task is asking for the result of the expression (3*5) + 2. The provided information shows only a sequence of calculations without a final answer. So, the task has not been completed.</think>
<completed>no</completed>
```

```
3*5 = 15
15 + 2 = 17
<task>Compute the result of (3*5) + 2?</task>
<think>Alright, so I need to evaluate whether the task "Compute the result of (3*5) + 2?" has been completed or not.
The task is asking for the result of the expression (3*5) + 2. The provided information shows the calculations leading to the final answer. So, the task has been completed successfully.</think>
<completed>yes</completed>
```

"""
                ),
                UserMessage(f"<task>{self.initial_query}</task>\n"),
                AssistantMessage(
                    f"""<think>Alright, so I need to evaluate whether the task "{self.initial_query}" has been completed or not. """
                ),
            ]
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
            return

        step_messages.extend(
            [
                SystemMessage(
                    """
# Next Step Guidance

You are a specialized assistant, you only need to figure out ONLY the next logical step as a suggestion to the user.
The task is for someone else to complete, and you are not responsible for executing it.

## Your Responsibilities:
- Keep step abstract and conceptual - avoid specific implementations or calculations
- Focus on WHAT needs to be done, not HOW to do it


For example:

```
<task>Create me a new recipe for a cake.</task>
<think>Alright, so I need to figure out the next logical step for the task "Create me a new recipe for a cake."
The first step would be to identify the type of cake to be created based on user preferences.</think>
Identify the type of cake to be created based on user preferences.

I want a recipie for an apple pie, with a flaky crust and a sweet filling.
<think>Alright, so I need to determine the next logical step for the task "Create me a new recipe for a cake."
We do know that the user wants an apple pie, so the next step would be to understand what the main ingredients are for the apple pie.</think>
Understand what the main ingredients are for the apple pie.

The main ingredients are apples, flour, sugar, and butter.
...
```

```
<task>Help me build a new website.</task>
<think>Alright, so I need to determine out the next logical step for the task "Help me build a new website."
The first step would be to identify the purpose and target audience of the website.</think>
Identify the purpose and target audience of the website.

The website is for a personal blog about travel experiences. I want it to be visually appealing and easy to navigate.
<think>Alright, so I need to provide the next logical step for the task "Help me build a new website."
We know that the website is for a personal blog about travel experiences, so the next step would be to determine the key features and content that should be included on the website.</think>
Determine the key features and content that should be included on the website.

The key features include a homepage, blog section, about page, and contact form. The content will focus on travel stories, tips, and photos.
...
```
"""
                ),
                UserMessage(f"<task>{self.initial_query}</task>\n"),
                AssistantMessage(
                    # we are "forcing" the model into thinking exactly about the next step
                    f"""<think>Alright, so I need to figure out the next logical step for the task "{self.initial_query}"\n"""
                ),
            ]
        )

        async for chunk in complete_think_chat(
            self.model,
            step_messages,
            self.stream,
            Phase.THINKING,
        ):
            yield chunk


class DynamicPlanner(Planner):
    def __init__(self, model: Model = None):
        if model is None:
            model = Model(name="deepseek-r1")

        self.model = model

    async def plan(
        self, messages: list[ConversationMessage], contextual_knowledge: Optional[str], stream: bool = False
    ) -> Plan:
        if messages[-1].role != ConversationRole.USER:
            raise Exception(f"Role {messages[-1].role} is not supported in the last message, only 'USER' is allowed.")

        return DynamicPlan(self.model, messages[-1].content, stream)
