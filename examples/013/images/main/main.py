from ockam import Agent, Model, Node, Tool

from playwright.async_api import async_playwright
from datetime import datetime


class Browser:
    def __init__(self, playwright, browser):
        self.playwright = playwright
        self.browser = browser

    async def new_context(self):
        return await self.browser.new_context()

    async def stop(self):
        await self.browser.close()
        await self.playwright.stop()

    @staticmethod
    async def start(address):
        playwright = await async_playwright().start()
        browser = await playwright.chromium.connect(address)
        return Browser(playwright, browser)


class BrowserSession:
    def __init__(self, browser, context, page):
        self.browser = browser
        self.context = context
        self.page = page

    def tools(self):
        async def goto(url: str) -> dict:
            """
            Go to the given url.
            """
            await self.page.goto(url)
            return {"status": "ok", "url": self.page.url}

        async def title(limit: int) -> dict:
            """
            Get the title of the current page.
            """
            title = await self.page.title()
            return {"status": "ok", "title": title}

        async def content(limit: int) -> dict:
            """
            Get the content of the current page.
            """
            content = await self.page.content()
            return {"status": "ok", "content": content}

        async def fill(css_selector: str, value: str) -> dict:
            """
            Select an input field using the given css_selector.
            Then fill that input field with the given value.
            """
            await self.page.locator(css_selector).fill(value)
            return {"status": "ok"}

        async def enter() -> dict:
            """
            Press enter anywhere on the page.
            """
            await self.page.keyboard.press('Enter')
            return {"status": "ok"}

        async def click(css_selector: str) -> dict:
            """
            Select an element using the given css_selector.
            Then click on that element.
            """
            await self.page.locator(css_selector).click()
            return {"status": "ok"}

        async def text_content_of_element(css_selector: str) -> dict:
            """
            Select an element using the given css_selector.
            Then extract the textContent on that element.
            """
            content = await self.page.locator(css_selector).textContent()
            return {"status": "ok", "content": content}

        async def screenshot() -> dict:
            """
            Take a screenshot of the current page.
            """
            screenshot_bytes = await self.page.screenshot()
            return {"status": "ok", "screenshot": screenshot_bytes}

        async def current_iso8601_utc_time() -> str:
            """
            Returns the current UTC time in ISO 8601 format.
            """
            return datetime.utcnow().isoformat() + "Z"

        return [
            Tool(goto),
            Tool(title),
            Tool(content),
            Tool(fill),
            Tool(enter),
            Tool(click),
            Tool(text_content_of_element),
            Tool(screenshot),
            Tool(current_iso8601_utc_time)
        ]

    @staticmethod
    async def start(browser):
        context = await browser.new_context()
        page = await context.new_page()
        return BrowserSession(browser, context, page)


async def main(node):
    browser = await Browser.start("ws://localhost:3333/")
    browser_session = await BrowserSession.start(browser)
    tools = browser_session.tools()

    agent = await Agent.start(
        node=node,
        name="jack",
        model=Model("nova-lite-v1"),
        tools=tools,
        instructions="""
            You are a web browsing agent.

            - You have in depth knowledge of HTML and how it is structured.
            - You are an expert at writing css queries to select elements on a web page.
            - You have been given a set of tools that can drive the actions of a web browser.
            - Use these tools to navigate and interact with web pages.
            - Once you've completed a request, summarize the actions you took in a single sentence.
            - If you are unable to complete a request, explain why in a single sentence.
        """,
    )


Node.start(main)
