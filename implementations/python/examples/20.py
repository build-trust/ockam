from ockam import Agent, Model, Node

"""
    This example shows how a thinking model shows the reasoning process.
"""


async def main(node):
    agent = await Agent.start(
        node=node,
        name="assistant",
        instructions="You are an assistant.",
        model=Model(name="deepseek-r1"),
    )

    messages = await agent.send("Create python code that prints 'Hello World'.")
    for message in messages:
        if message.thinking:
            print(f"\033[3;94m{message.content}\033[0m", end="")
        else:
            print(f"{message.content}", end="")


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/20.py
