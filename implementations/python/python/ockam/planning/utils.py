from ..models import Model
from ockam.nodes.message import Phase, UserMessage


async def complete_think_chat(model: Model, step_messages: list, stream: bool, initial_phase: Phase = Phase.PLANNING):
    expect_reasoning_content = False
    phase = initial_phase
    if stream:
        async for chunk in await model.complete_chat(
            messages=step_messages,
            temperature=0,
            stream=True,
        ):
            content = chunk.choices[0].delta.content
            if content is not None:
                if "<think>" in content:
                    phase = Phase.THINKING
                if "</think>" in content:
                    phase = Phase.PLANNING
                content = content.replace("<think>", "").replace("</think>", "")

            if expect_reasoning_content and not hasattr(chunk.choices[0].delta, "reasoning_content"):
                phase = Phase.PLANNING

            if (
                hasattr(chunk.choices[0].delta, "reasoning_content")
                and len(chunk.choices[0].delta.reasoning_content) > 0
            ):
                expect_reasoning_content = True
                phase = Phase.THINKING
                content = chunk.choices[0].delta.reasoning_content

            if content is not None and len(content) > 0:
                yield UserMessage(content, phase=phase)
    else:
        response = await model.complete_chat(
            messages=step_messages,
            temperature=0,
            stream=False,
        )
        text = response.choices[0].message.content
        if "</think>" in text:
            pieces = text.split("</think>")
            think_text = pieces[0].replace("<think>", "")
            yield UserMessage(think_text, phase=Phase.THINKING)
            text = pieces[1]

        if hasattr(response, "reasoning_content") and len(response.reasoning_content) > 0:
            reasoning_content = response.reasoning_content
            yield UserMessage(reasoning_content, phase=Phase.THINKING)

        yield UserMessage(text, phase=Phase.PLANNING)
