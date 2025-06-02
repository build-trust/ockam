from ockam import Node, info

"""
    Second part of example 04. This simply starts an Echoer worker.
"""


class Echoer:
    async def handle_message(self, context, message):
        info(f"Echoer received: {message}")
        await context.reply(message)


async def main(node):
    await node.start_worker("echoer", Echoer())


Node.start(main)

# OCKAM_SQLITE_IN_MEMORY=1 CLUSTER=acme NODE=node2 ENROLLMENT_TICKET="$(ockam project ticket --relay node2 --attribute cluster=acme)" uv run examples/04-echo.py
