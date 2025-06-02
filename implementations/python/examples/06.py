from ockam import Agent, Model, Node, Tool

"""
    This example shows how to add tools to an agent.
    In this case we add two simple functions. The agent will use the model to learn how to use the provided tools.
"""


def divide(a: float, b: float) -> float:
    a = float(a)
    b = float(b)
    return a / b


def multiply(a: float, b: float) -> float:
    a = float(a)
    b = float(b)
    return a * b


async def main(node):
    agent = await Agent.start(
        node=node,
        name="Calculator",
        instructions="You are an assistant that can multiply or divide two numbers.",
        model=Model(name="ollama_chat/llama3.2"),
        tools=[Tool(divide), Tool(multiply)],
    )
    reply = await agent.send("What is 56 divided by 27?")
    print(reply)
    reply = await agent.send("What is 214 multiplied by 63?")
    print(reply)


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/06.py
