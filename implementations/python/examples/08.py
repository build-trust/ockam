from ockam import Agent, Model, Node, CoTPlanner
from ockam.nodes.message import Phase

"""
    This example shows how to use a planning strategy ("Chain of Thought" or COT) to solve a complex task.
    As you can see, the planning phase can use a different model than the execution phase
"""


async def main(node):
    agent = await Agent.start(
        node=node,
        name="Assistant",
        instructions="Assistant to solve some complex task ...",
        model=Model(name="ollama_chat/llama3.2"),
        planner=CoTPlanner(),
    )

    async for chunk in agent.send_stream("Estimate how many violins are in the world", timeout=60 * 20):
        if len(chunk.snippet.messages) == 0:
            continue

        phase = chunk.snippet.messages[-1].phase
        text = chunk.snippet.messages[-1].content

        if phase == Phase.THINKING:
            print(f"\033[94m{text}\033[0m", end="")  # thinking: blue
        elif phase == Phase.PLANNING:
            print(f"\033[92m{text}\033[0m", end="")  # planning: green
        elif phase == Phase.EXECUTING:
            print(f"\033[90m{text}\033[0m", end="")  # executing: grey
        else:
            raise RuntimeError(f"unknown phase {phase}")


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/08.py
