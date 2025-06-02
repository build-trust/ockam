from .protocol import TextExtractor


class PlainTextExtractor(TextExtractor):
    async def extract_text(self, input_data: str | bytes, content_type: str = None) -> str:
        if isinstance(input_data, bytes):
            # TODO: handle detecting and using encodings
            return input_data.decode("utf-8")
        return input_data
