from ..models import Model
from ockam.nodes.message import Phase, UserMessage


async def complete_think_chat(model: Model, step_messages: list, stream: bool):
    thinking = False
    if stream:
        async for chunk in await model.complete_chat(
            messages=step_messages,
            is_thinking=True,
            temperature=0,
            stream=True,
        ):
            content = chunk.choices[0].delta.content

            if hasattr(chunk.choices[0].delta, "reasoning_content") and chunk.choices[0].delta.reasoning_content:
                thinking = True
                content = chunk.choices[0].delta.reasoning_content
            else:
                thinking = False

            if content is not None and len(content) > 0:
                yield UserMessage(content, phase=Phase.PLANNING, thinking=thinking)
    else:
        response = await model.complete_chat(
            messages=step_messages,
            temperature=0,
            stream=False,
        )
        message = response.choices[0].message

        if hasattr(message, "reasoning_content") and message.reasoning_content:
            yield UserMessage(message.reasoning_content, phase=Phase.PLANNING, thinking=True)

        yield UserMessage(message.content, phase=Phase.PLANNING, thinking=False)
