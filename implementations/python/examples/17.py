from ockam import Agent, Model, Node, Squad

"""
    This example shows how squads work
"""


async def main(node):
    squad = Squad()
    for i in range(5):
        agent = await Agent.start(
            node=node,
            name=f"Assistant-{i}",
            instructions="Assistant to solve some complex task ...",
            model=Model(name="ollama_chat/llama3.2"),
        )
        squad.add(agent, "Write a haiku")

    results = await squad.run()
    print(results)


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/17.py
