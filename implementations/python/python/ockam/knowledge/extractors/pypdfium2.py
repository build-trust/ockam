from .protocol import TextExtractor

import pypdfium2
import threading
import asyncio


# Only one thread can concurrently use pypdfium2, this is an upstream limitation.
LOCK = threading.Lock()


class PdfTextExtractor(TextExtractor):
    def __init__(self):
        pass

    async def extract_text(self, input_data: str | bytes, content_type: str = None) -> str:
        return await asyncio.to_thread(self._sync_extract_text, input_data, content_type)

    def _sync_extract_text(self, input_data: str | bytes, _content_type: str = None) -> str:
        global LOCK
        with LOCK:
            pdf = pypdfium2.PdfDocument(input_data)
            text = []
            for page in pdf:
                text.append(page.get_textpage().get_text_range())
            return "\n".join(text)
