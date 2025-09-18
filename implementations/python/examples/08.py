from ockam import Agent, Model, Node, CoTPlanner
from ockam.nodes.message import Phase, MessageContentType

"""
    This example shows how to use a planning strategy ("Chain of Thought" or COT) to solve a complex task.
    As you can see, the planning phase can use a different model than the execution phase
"""


async def main(node):
    agent = await Agent.start(
        node=node,
        name="Assistant",
        instructions="Assistant to solve some complex task ...",
        model=Model(name="deepseek-r1"),
        planner=CoTPlanner(model=Model(name="deepseek-r1")),
    )

    async for chunk in agent.send_stream("Estimate how many violins are in the world", timeout=60 * 20):
        if len(chunk.snippet.messages) == 0:
            continue

        phase = chunk.snippet.messages[-1].phase
        thinking = chunk.snippet.messages[-1].thinking
        text = chunk.snippet.messages[-1].content.text

        # reasoning is always in italic
        if phase == Phase.PLANNING:  # planning: green
            if thinking:
                print(f"\033[3;92m{text}\033[0m", end="")
            else:
                print(f"\033[92m{text}\033[0m", end="")
        elif phase == Phase.EXECUTING:  # executing: grey
            if thinking:
                print(f"\033[3;90m{text}\033[0m", end="")
            else:
                print(f"\033[90m{text}\033[0m", end="")
        else:
            raise RuntimeError(f"unknown phase {phase}")


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/08.py
