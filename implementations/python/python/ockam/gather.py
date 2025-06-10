import asyncio


def gather(*coros_or_futures, batch_size=None, timeout=None, return_exceptions=False):
    if not batch_size:
        return asyncio.gather(*coros_or_futures)

    sem = asyncio.Semaphore(batch_size)

    async def batch_task(f):
        async with sem:
            if not timeout:
                return await f
            else:
                return await asyncio.wait_for(f, timeout)

    futures = [batch_task(f) for f in coros_or_futures]

    return asyncio.gather(*futures, return_exceptions=return_exceptions)
