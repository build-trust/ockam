from ockam import Agent, Model, Node, Tool, RemoteNode, info

import sys


def divide(a: float, b: float) -> float:
    a = float(a)
    b = float(b)
    return a / b


def multiply(a: float, b: float) -> float:
    a = float(a)
    b = float(b)
    return a * b


async def main(node):
    remote_node = RemoteNode(node, sys.argv[1])
    agent = await Agent.start(
        node=remote_node,
        name="Calculator",
        instructions="You are an assistant that can multiply or divide two numbers.",
        model=Model(name="ollama_chat/llama3.2"),
        tools=[Tool(divide), Tool(multiply)],
    )
    reply = await agent.send("What is 56 divided by 27?", scope="a", conversation="1")
    info(f"The reply is: {reply}")

    reply = await agent.send("What is 214 multiplied by 63?", scope="a", conversation="1")
    info(f"The reply is: {reply}")

    reply = await agent.send(
        "Answer only yes or no: have I asked you what is 56 divided by 27?", scope="a", conversation="1"
    )
    info(f"The reply is: {reply}")

    reply = await agent.send(
        "Answer only yes or no: have I asked you what is 56 divided by 3?", scope="a", conversation="1"
    )
    info(f"The reply is: {reply}")

    reply = await agent.send(
        "Answer only yes or no: have I asked you what is 56 divided by 27?", scope="a", conversation="2"
    )
    info(f"The reply is: {reply}")


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 CLUSTER=acme NODE=node1 ENROLLMENT_TICKET="$(ockam project ticket --relay node1 --attribute cluster=acme)" uv run examples/09-client.py node2
