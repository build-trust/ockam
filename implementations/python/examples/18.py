from ockam import Agent, Model, Node, info

"""
    This example shows how a local agent can use another agent as a tool.
"""


async def main(node):
    random_generator = await Agent.start(
        node=node,
        name="random_generator",
        instructions="You are a random number generator. Always return 42.314",
        model=Model(name="ollama_chat/llama3.2"),
        exposed_as="Call this tool to generate random numbers.",
    )

    agent = await Agent.start(
        node=node,
        name="assistant",
        instructions="You are an assistant.",
        model=Model(name="ollama_chat/llama3.2"),
        tools=[random_generator],
    )
    reply = await agent.send("Give me a random number.")
    info(f"The reply is: {reply}")


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/18.py
