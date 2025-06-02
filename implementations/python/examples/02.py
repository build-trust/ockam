from ockam import Node, info

"""
    This example shows that it is possible to start a worker written in Python,
    send a message to it, and get a response.

    The node is started with a name and relay, which means that an identity will be created for that node.
    and it will be enrolled to the project mentioned in the ticket.
"""


class Echoer:
    async def handle_message(self, context, message):
        info(f"Echoer received: {message}")
        await context.reply(message)


async def main(node):
    await node.start_worker("echoer", Echoer())
    reply = await node.send_and_receive(destination="echoer", message="hello")
    info(f"Reply received: {reply}")


Node.start(main, wait_until_interrupted=False)

# OCKAM_SQLITE_IN_MEMORY=1 uv run examples/02.py
