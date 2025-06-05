from ockam import Agent, Node, CoTPlanner

"""
Ask a question to this agent via HTTP:
```
http --stream -b POST ':8000/agents/henry?timeout=60' message="Estimate how many violins there are in the world"
```

"""
async def main(node):
    await Agent.start(
        node=node,
        name="henry",
        instructions="You are an assistant who solves complex tasks by planning them carefully before solving them.",
        planner=CoTPlanner(),
    )



Node.start(main)
