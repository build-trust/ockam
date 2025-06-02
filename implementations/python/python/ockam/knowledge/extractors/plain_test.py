from .plain import PlainTextExtractor


async def test_plain_and_binary():
    extractor = PlainTextExtractor()
    text = await extractor.extract_text("Hello, World!")
    assert text == "Hello, World!"
    text = await extractor.extract_text(b"Hello, World!")
    assert text == "Hello, World!"
