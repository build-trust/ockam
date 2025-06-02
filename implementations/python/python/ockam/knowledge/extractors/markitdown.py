import io
import asyncio

from markitdown import MarkItDown
from ockam.knowledge.extractors import TextExtractor


class MarkItDownTextExtractor(TextExtractor):
    async def extract_text(self, input_data: str | bytes, content_type: str = None) -> str:
        return await asyncio.to_thread(self._sync_extract_text, input_data, content_type)

    def _sync_extract_text(self, input_data: str | bytes, content_type: str = None) -> str:
        if isinstance(input_data, str):
            input_data = input_data.encode("utf-8")

        stream = io.BytesIO(input_data)
        md = MarkItDown()
        return md.convert(source=stream).markdown
