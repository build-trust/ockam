import asyncio

from ockam import InfoContext


async def call_agent(agent, query):
    return await agent.send(query)


class Squad(InfoContext):
    _logger = None

    @classmethod
    def class_logger(cls):
        if cls._logger:
            return cls._logger
        else:
            from ..logging.logging import get_logger

            cls._logger = get_logger("squad")
            return cls._logger

    def __init__(self):
        self.logger = Squad.class_logger()
        self.coroutines = []

    def add(self, agent, query, portals=[]):
        self.coroutines.append(call_agent(agent, query))

    async def run(self):
        with self.info("Running squad", "Squad run completed"):
            async with asyncio.TaskGroup() as tg:
                tasks = [tg.create_task(coro) for coro in self.coroutines]
            results = [task.result() for task in tasks]
            self.logger.debug("Squad results: %s", results)
            return results
