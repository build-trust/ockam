from ockam import Agent, Model, Node, ReActPlanner


async def main(node):
    await Agent.start(
        node=node,
        name="henry",
        instructions="You are Henry, an expert legal assistant.",
        model=Model("llama3.2"),
        planner=ReActPlanner(Model("deepseek-r1")),
    )


Node.start(main)
