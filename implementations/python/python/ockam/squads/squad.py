import asyncio


async def call_agent(agent, query):
    return await agent.send(query)


class Squad:
    def __init__(self):
        self.coroutines = []

    def add(self, agent, query, portals=[]):
        self.coroutines.append(call_agent(agent, query))

    async def run(self):
        async with asyncio.TaskGroup() as tg:
            tasks = [tg.create_task(coro) for coro in self.coroutines]
        return [task.result() for task in tasks]
