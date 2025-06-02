import asyncio


def gather(*coros_or_futures, batch_size=None):
    if not batch_size:
        return asyncio.gather(*coros_or_futures)

    sem = asyncio.Semaphore(batch_size)

    async def batch_task(f):
        async with sem:
            return await f

    futures = [batch_task(f) for f in coros_or_futures]

    return asyncio.gather(*futures)
