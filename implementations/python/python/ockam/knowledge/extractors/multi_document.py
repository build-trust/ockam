from typing import Union, Optional
from .protocol import TextExtractor
from .pypdfium2 import PdfTextExtractor
from .plain import PlainTextExtractor

import filetype
from ...ockam_in_rust_for_python import warn


class MultiDocumentExtractor(TextExtractor):
    fail_on_error: bool

    def __init__(self, fail_on_error: bool = False):
        """
        Initializes the MultiDocumentExtractor.
        :param fail_on_error: If True, raises an exception on error; otherwise, logs a warning.
        """
        self.fail_on_error = fail_on_error

    def guess_content_type(self, input_data: Union[str, bytes]) -> Optional[str]:
        """
        Guess the content type of the input data.

        :param input_data: The input data to guess the content type for.
        :return: The guessed content type, or None if it couldn't be determined.
        """
        if isinstance(input_data, str):
            # a string would be considered as a file path
            raw_bytes: bytes = input_data.encode("utf-8")
        else:
            raw_bytes = input_data

        content_type: Optional[str] = None
        try:
            content_type = filetype.guess_mime(raw_bytes)
        except Exception:
            pass

        if content_type is None:
            text_sample: Optional[str] = None

            if isinstance(input_data, bytes):
                try:
                    # If all the content is valid utf-8, we can assume its plain text
                    text: str = input_data.decode("utf-8", errors="strict")
                    text_sample = text.strip()[:20].lower()
                except UnicodeDecodeError:
                    content_type = None

            if isinstance(input_data, str):
                text_sample = input_data.strip()[:20].lower()

            if text_sample is not None:
                if "html" in text_sample:
                    content_type = "text/html"
                elif text_sample.startswith("<"):
                    content_type = "application/xml"
                elif text_sample.startswith("{") or text_sample.startswith("["):
                    content_type = "application/json"
                else:
                    content_type = "text/plain"
        return content_type

    async def extract_text(self, input_data: Union[str, bytes], content_type: Optional[str] = None) -> str:
        """
        Extract text from the input data.

        :param input_data: The input data to extract text from.
        :param content_type: The content type of the input data, or None to guess it.
        :return: The extracted text.
        :raises ValueError: If fail_on_error is True, and the content type is unsupported or couldn't be determined.
        """
        if content_type is None:
            content_type = self.guess_content_type(input_data)

        extractor: TextExtractor
        match content_type:
            case "application/pdf":
                extractor = PdfTextExtractor()
            case "text/plain" | "text/markdown" | "application/json":
                extractor = PlainTextExtractor()
            case None:
                if self.fail_on_error:
                    raise ValueError("Unable to determine content type for input data.")
                else:
                    warn("Unable to determine content type for input data, skipping content extraction.")
                    return ""
            case _:
                if self.fail_on_error:
                    raise ValueError(f"Unsupported content type: {content_type}")
                else:
                    warn(f"Unsupported content type: {content_type}, skipping content extraction.")
                    return ""

        return await extractor.extract_text(input_data, content_type)
