from ockam import Agent, Model, Node, Tool, info

"""
    This example shows the difference between querying a model using local tools and
    using a remote agent to do the same kind of querying on a remote node (files 09-client.py and 09-server.py).
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
    reply = await agent.send("What is 56 divided by 27?", scope="a", conversation="1")
    info(f"What is 56 divided by 27? {reply}")

    reply = await agent.send("What is 214 multiplied by 63?", scope="a", conversation="1")
    info(f"What is 214 multiplied by 63? {reply}")

    reply = await agent.send(
        "Answer only yes or no: have I asked you what is 56 divided by 27?", scope="a", conversation="1"
    )
    info(f"Answer only yes or no: have I asked you what is 56 divided by 27? {reply}")

    reply = await agent.send(
        "Answer only yes or no: have I asked you what is 56 divided by 3?", scope="a", conversation="1"
    )
    info(f"Answer only yes or no: have I asked you what is 56 divided by 3? {reply}")

    reply = await agent.send(
        "Answer only yes or no: have I asked you what is 56 divided by 27?", scope="a", conversation="2"
    )
    info(f"Answer only yes or no: have I asked you what is 56 divided by 27? {reply}")


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/09.py
