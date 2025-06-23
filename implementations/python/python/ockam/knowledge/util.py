import aiofiles
import aiohttp
from urllib.parse import urlparse


async def download_url(url: str) -> bytes:
    parsed_url = urlparse(url)
    match parsed_url.scheme:
        case "http" | "https":
            async with aiohttp.ClientSession() as session:
                async with session.get(url) as response:
                    response.raise_for_status()
                    return await response.read()
        case "file" | "":
            async with aiofiles.open(parsed_url.path, mode="rb") as f:
                return await f.read()
    raise ValueError(f"Unsupported URL scheme: {parsed_url.scheme}. Supported schemes are 'http', 'https', and 'file'.")
