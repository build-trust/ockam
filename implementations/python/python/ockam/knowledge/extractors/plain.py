from typing import Union, Optional
from .protocol import TextExtractor


class PlainTextExtractor(TextExtractor):
    async def extract_text(self, input_data: Union[str, bytes], content_type: Optional[str] = None) -> str:
        if isinstance(input_data, bytes):
            # TODO: handle detecting and using encodings
            return input_data.decode("utf-8")
        return input_data
