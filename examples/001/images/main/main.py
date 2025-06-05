from ockam import Agent, Node


async def main(node):
    await Agent.start(node, "You are Henry, an expert legal assistant", "henry")


Node.start(main)
