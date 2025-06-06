from ockam.ockam_in_rust_for_python import debug

from .multi_document import MultiDocumentExtractor
from .protocol import TextExtractor


def create_extractor() -> TextExtractor:
    """
    Factory function to create an extractor.
    """

    # try to use markitdown if available
    try:
        from .markitdown import MarkItDownTextExtractor

        debug("Using markitdown for text extraction")
        return MarkItDownTextExtractor()
    except ImportError:
        pass

    debug("Using internal text extraction")
    # use the standard multi-document extractor
    return MultiDocumentExtractor()


__all__ = [
    "TextExtractor",
    "create_extractor",
]
