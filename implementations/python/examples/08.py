from ockam import Agent, Model, Node, CoTPlanner

"""
  This example shows how to use a planning strategy ("Chain of Thought" or COT) to solve a complex task.
  As you can see the planning phase can use a different model than the execution phase
"""


async def main(node):
    agent = await Agent.start(
        node=node,
        name="Assistant",
        instructions="Assistant to solve some complex task ...",
        model=Model(name="ollama_chat/llama3.2"),
        planner=CoTPlanner(),
    )
    reply = await agent.send("Estimate how many violins are in the world", timeout=60 * 2)
    print(reply)


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/08.py
