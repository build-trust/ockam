import litellm

from ockam import Agent, Model, Node
from ockam.nodes.message import UserMessage, ImageContent
from urllib import request

"""
    This example shows how to use an image in a conversation with an agent.
"""


async def main(node):
    agent = await Agent.start(
        node=node,
        name="assistant",
        instructions="You are an art critic",
        model=Model(name="llama3.2-vision"),
    )
    litellm._turn_on_debug()

    messages = await agent.send(
        [
            UserMessage(content=ImageContent(binary=request.urlopen("https://picsum.photos/id/237/1920/1080").read())),
            UserMessage("Describe the photo"),
        ],
        timeout=1000,
    )
    for message in messages:
        if message.thinking:
            print(f"\033[3;94m{message.content}\033[0m", end="")
        else:
            print(f"{message.content}", end="")


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/21.py
