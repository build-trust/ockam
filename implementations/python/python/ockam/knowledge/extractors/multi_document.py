from .protocol import TextExtractor
from .pypdfium2 import PdfTextExtractor
from .plain import PlainTextExtractor

import filetype
from ...ockam_in_rust_for_python import warn


class MultiDocumentExtractor(TextExtractor):
    def __init__(self, fail_on_error: bool = False):
        """
        Initializes the MultiDocumentExtractor.
        :param fail_on_error: If True, raises an exception on error; otherwise, logs a warning.
        """
        self.fail_on_error = fail_on_error

    def guess_content_type(self, input_data: str | bytes) -> str | None:
        if isinstance(input_data, str):
            # a string would be considered as a file path
            raw_bytes = input_data.encode("utf-8")
        else:
            raw_bytes = input_data

        content_type = None
        try:
            content_type = filetype.guess_mime(raw_bytes)
        except Exception:
            pass

        if content_type is None:
            text_sample = None

            if isinstance(input_data, bytes):
                try:
                    # If all the content is valid utf-8, we can assume its plain text
                    text = input_data.decode("utf-8", errors="strict")
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

    async def extract_text(self, input_data: str | bytes, content_type: str = None) -> str:
        if content_type is None:
            content_type = self.guess_content_type(input_data)

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
