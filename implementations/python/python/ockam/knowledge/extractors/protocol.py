from typing import Protocol, Union, Optional


class TextExtractor(Protocol):
    async def extract_text(self, input_data: Union[str, bytes], content_type: Optional[str] = None) -> str:
        """
        Extract text from a PDF file.

        :param content_type: Optional content type of the input data, used for determining how to process the input.
        :param input_data: Path to the PDF file or bytes-like object containing PDF data.
        :return: Extracted text from the PDF.
        """
        ...
