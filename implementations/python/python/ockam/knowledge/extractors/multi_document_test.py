from . import MultiDocumentExtractor


def test_guess():
    extractor = MultiDocumentExtractor()
    assert "text/plain" == extractor.guess_content_type("Hello, World!")
    assert "text/plain" == extractor.guess_content_type(b"Hello, World!")
    assert "application/json" == extractor.guess_content_type('{"key": "value"}')
    assert "application/json" == extractor.guess_content_type(b'{"key": "value"}')
    assert "application/json" == extractor.guess_content_type('["item1", "item2"]')
    assert "text/html" == extractor.guess_content_type("<html><body>Hello, World!</body></html>")
    assert "application/xml" == extractor.guess_content_type("<root><child>Hello, World!</child></root>")
    assert "application/pdf" == extractor.guess_content_type(b"%PDF-1.4\n")


async def test_flow():
    extractor = MultiDocumentExtractor()
    text = await extractor.extract_text("Hello, World!")
    assert text == "Hello, World!"


async def test_invalid_content_type():
    extractor = MultiDocumentExtractor()
    text = await extractor.extract_text("Hello, World!", content_type="application/unknown")
    assert text == ""

    extractor = MultiDocumentExtractor(fail_on_error=True)
    try:
        await extractor.extract_text("Hello, World!", content_type="application/unknown")
    except ValueError as e:
        assert str(e) == "Unsupported content type: application/unknown"


async def test_unknown_content_type():
    extractor = MultiDocumentExtractor()
    text = await extractor.extract_text("weird binary!!".encode("utf-16"))
    assert text == ""

    extractor = MultiDocumentExtractor(fail_on_error=True)
    try:
        await extractor.extract_text(b"unknown!")
    except ValueError as e:
        assert str(e) == "Unable to determine content type for input data."
