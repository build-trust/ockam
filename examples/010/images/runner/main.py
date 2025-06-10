from ockam import Node, WorkerProtocol, ContextProtocol, LocalNode
import asyncio

async def stats_loop():
    from ockam.models.model import print_requests_stats_by_model
    import asyncio
    while True:
        await asyncio.sleep(1)
        print_requests_stats_by_model()


class Stats(WorkerProtocol):
    async def handle_message(self, context: ContextProtocol, message: str):
        from ockam.models.model import csv_requests_stats_by_model
        await context.reply(csv_requests_stats_by_model())

async def main(node: LocalNode):
    await node.start_worker("stats", Stats())

Node.start(main)
