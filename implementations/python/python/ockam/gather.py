import asyncio


def gather(*coros_or_futures, batch_size=None, timeout=None, return_exceptions=False):
    sem = None
    if batch_size:
        sem = asyncio.Semaphore(batch_size)

    async def wrap_task(f):
        if timeout:
            f = asyncio.wait_for(f, timeout)

        if sem:
            async with sem:
                return await f
        else:
            return await f

    futures = [wrap_task(f) for f in coros_or_futures]

    return asyncio.gather(*futures, return_exceptions=return_exceptions)
