from .markitdown import MarkItDownTextExtractor


async def test_plain_text():
    extractor = MarkItDownTextExtractor()

    text = await extractor.extract_text(b"# Hello, World!")
    assert text == "# Hello, World!"

    text = await extractor.extract_text("<html><h1>Hello, World!</h1></html>")
    assert text == "# Hello, World!"

    text = await extractor.extract_text("<html><body>Hello, World!</body></html>")
    assert text == "Hello, World!"

    text = await extractor.extract_text('{ "key": "value" }')
    assert text == '{ "key": "value" }'
