from datetime import datetime
from sys import argv
from ockam import Agent, Model, Node, Tool, set_log_levels
from playwright.async_api import async_playwright

import asyncio
import re

# set_log_levels("agent=debug,ockam_node=info,ockam=info,LiteLLM=DEBUG")
set_log_levels("DEBUG")


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
    def __init__(self, browser, context):
        self.browser = browser
        self.context = context
        self.pages = []
        self.locators = []

    def tools(self):

        async def bring_to_front(page_index: int = 0) -> dict:
            """
            Brings the page at the given index to the front.
            """
            await self.pages[page_index].bring_to_front()
            return {"status": "ok"}

        async def click(page_index: int = 0) -> dict:
            """
            Clicks on the locator for the page at the given page index.
            """
            await self.locators[page_index].click()
            return {"status": "ok"}

        async def click_nth(page_index: int = 0, nth: int = 1) -> dict:
            """
            Clicks on the nth element that was matched by the locator
            for the page at the given page index.
            """
            nth = nth - 1
            await self.locators[page_index].nth(nth).click()
            return {"status": "ok"}

        async def current_iso8601_utc_time() -> str:
            """
            Returns the current UTC time in ISO 8601 format.
            """
            return datetime.utcnow().isoformat() + "Z"

        async def enter(page_index: int = 0) -> dict:
            """
            Press enter anywhere on the page at the given page index.
            """
            await self.pages[page_index].keyboard.press('Enter')
            await self.pages[page_index].wait_for_load_state("domcontentloaded")
            await asyncio.sleep(2)
            return {"status": "ok"}

        async def fill(value: str, page_index: int = 0) -> dict:
            """
            Fills the element specified by the current locator
            for the page at the given index with the given value.
            """
            await self.locators[page_index].fill(value)
            return {"status": "ok"}

        async def fill_active(value: str, page_index: int = 0) -> dict:
            """
            Fills the active element on the page at the given
            index with the given value.

            Returns true if the element was filled, false otherwise.
            """
            page = self.pages[page_index]
            active = await page.evaluate_handle("() => document.activeElement")
            element = active.as_element()
            if element:
                await element.fill(value)
                return {"status": "ok", "filled": True}

            selector = page.locator("input[autofocus]")
            count = await selector.count()
            if count == 1:
                await selector.fill(value)
                return {"status": "ok", "filled": True}

            return {"status": "ok", "filled": False}

        async def filter(text: str, page_index: int = 0) -> dict:
            """
            Filters the current locator for the page at the given index
            by applying a has_text filter to the current locator

            Returns the count of elements currently matched by the locator.
            """
            locator = self.locators[page_index].filter(has_text=text)
            count = await locator.count()
            if count == 0:
                return {"status": "error", "message": f"No elements found with text: {text}"}
            else:
                self.locators[page_index] = locator
                return {"status": "ok", "count": count}

        async def goto(url: str, page_index: int = 0) -> dict:
            """
            Go to the given url for the page at the given index and
            reset the locator on that page.
            """
            if page_index >= len(self.pages):
                result = await new_tab()
                page_index = result["page_index"]

            page = self.pages[page_index]
            # Ignore timeout errors when navigating
            try:
                await page.goto(url, timeout=5000)
            except Exception as e:
                None
            await asyncio.sleep(1)
            self.locators[page_index] = page.locator("body")
            return {"status": "ok", "url": page.url}

        async def locate(selector: str, page_index: int = 0) -> dict:
            """
            Sets the locator for the page at the given index
            with the given selector.

            Returns the count of elements currently matched by the locator.
            """
            locator = self.pages[page_index].locator(selector)
            count = await locator.count()
            if count == 0:
                return {"status": "error", "message": f"No elements found for selector: {selector}"}
            else:
                self.locators[page_index] = locator
                return {"status": "ok", "count": count}

        async def locate_by_css(selector: str, page_index: int = 0) -> dict:
            """
            Sets the locator for the page at the given index to the result
            of a css query on the page with the given css selector.

            Returns the count of elements currently matched by the locator.
            """
            locator = self.pages[page_index].locator(f"css={selector}")
            count = await locator.count()
            if count == 0:
                return {"status": "error", "message": f"No elements found for selector: {selector}"}
            else:
                self.locators[page_index] = locator
                return {"status": "ok", "count": count}

        async def locate_by_xpath(selector: str, page_index: int = 0) -> dict:
            """
            Sets the locator for the page at the given index to the result
            of a xpath query on the page with the given xpath selector.

            Returns the count of elements currently matched by the locator.
            """
            locator = self.pages[page_index].locator(f"xpath={selector}")
            count = await locator.count()
            if count == 0:
                return {"status": "error", "message": f"No elements found for selector: {selector}"}
            else:
                self.locators[page_index] = locator
                return {"status": "ok", "count": count}

        async def locate_by_role(role: str, page_index: int = 0) -> dict:
            """
            Sets the locator for the page at the given index to the result
            of get_by_role with the given role on the current page.

            Returns the count of elements currently matched by the locator.
            """
            locator = self.pages[page_index].get_by_role(role)
            count = await locator.count()
            if count == 0:
                return {"status": "error", "message": f"No elements found for role: {role}"}
            else:
                self.locators[page_index] = locator
                return {"status": "ok", "count": await locator.count()}

        async def new_tab() -> dict:
            """
            Opens a new tab in the current context.
            """
            new_page = await self.context.new_page()
            self.pages.append(new_page)
            self.locators.append(new_page)
            await new_page.bring_to_front()
            return {"status": "ok", "page_index": len(self.pages) - 1}

        async def search(query: str, page_index: int = 0) -> dict:
            """
            Performs a search for the given query on the page at the given index.
            """
            page = self.pages[page_index]

            locators = [
                (page.get_by_label, re.compile(r"search", re.IGNORECASE)),
                (page.locator, "input[name='q']"),
                (page.locator, "textarea[name='q']"),
                (page.locator, "input[type='search']"),
                (page.locator, "textarea[type='search']"),
                (page.get_by_placeholder, re.compile(r"search", re.IGNORECASE)),
                (page.get_by_role, "searchbox"),
            ]

            for func, selector in locators:
                input_locator = func(selector)
                count = await input_locator.count()
                if count == 1:
                    await input_locator.fill(query)
                    await input_locator.press('Enter')
                    await page.wait_for_load_state("domcontentloaded")
                    await asyncio.sleep(1)
                    return {"status": "ok", "message": "Search submitted"}

            return {"status": "error", "message": "Search input not found"}

        async def snapshot_full(page_index: int = 0) -> dict:
            """
            Takes a snapshot of the full page at the given index.

            Returns a list of all elements on the page along with
            details about each element.
            """
            page = self.pages[page_index]
            snapshot = await page.evaluate("""
                () => {
                function getRole(el) {
                    return el.getAttribute('role') || null;
                }

                function isVisible(el) {
                    const rect = el.getBoundingClientRect();
                    return rect.width > 0 && rect.height > 0;
                }

                function isInteractable(el) {
                    const tag = el.tagName.toLowerCase();
                    return ['button', 'a', 'input', 'select', 'textarea', 'label'].includes(tag) || getRole(el);
                }

                function getElementInfo(el) {
                    const rect = el.getBoundingClientRect();
                    return {
                        tag: el.tagName.toLowerCase(),
                        role: getRole(el),
                        text: el.innerText || el.value || '',
                        name: el.getAttribute('name') || '',
                        type: el.getAttribute('type') || '',
                        ariaLabel: el.getAttribute('aria-label') || '',
                        id: el.id || '',
                        class: el.className || '',
                        boundingBox: {
                            x: rect.x,
                            y: rect.y,
                            width: rect.width,
                            height: rect.height
                        }
                    };
                }

                const elements = Array.from(document.querySelectorAll('*'));
                return elements
                    .filter(el => isVisible(el) && isInteractable(el))
                    .map(el => getElementInfo(el));
                }
                """)
            return {"status": "ok", "snapshot": snapshot}

        async def snapshot_main(page_index: int = 0) -> dict:
            """
            Takes a snapshot of the main content of the page
            at the given index.

            Returns a list of all elements under the 'main' element
            on the page along with details about each element, or
            and empty list if no 'main' element is found.
            """
            page = self.pages[page_index]
            snapshot = await page.evaluate("""
                () => {
                function getRole(el) {
                    return el.getAttribute('role') || null;
                }

                function isVisible(el) {
                    const rect = el.getBoundingClientRect();
                    return rect.width > 0 && rect.height > 0;
                }

                function isInteractable(el) {
                    const tag = el.tagName.toLowerCase();
                    return ['button', 'a', 'input', 'select', 'textarea', 'label'].includes(tag) || getRole(el);
                }

                function getElementInfo(el) {
                    const rect = el.getBoundingClientRect();
                    return {
                        tag: el.tagName.toLowerCase(),
                        role: getRole(el),
                        text: el.innerText || el.value || '',
                        name: el.getAttribute('name') || '',
                        type: el.getAttribute('type') || '',
                        ariaLabel: el.getAttribute('aria-label') || '',
                        id: el.id || '',
                        class: el.className || '',
                        boundingBox: {
                            x: rect.x,
                            y: rect.y,
                            width: rect.width,
                            height: rect.height
                        }
                    };
                }

                let main = document.querySelector("main, #main, [role=main]")
                if (!main) return [];

                const elements = Array.from(main.querySelectorAll('*'));
                return elements
                    .filter(el => isVisible(el) && isInteractable(el))
                    .map(el => getElementInfo(el));
                }
                """)
            if not snapshot:
                return {"status": "error", "message": "No main content found"}
            else:
                return {"status": "ok", "snapshot": snapshot}

        async def text_contents(page_index: int = 0) -> dict:
            """
            Gets an array of the text contents of the elements
            matched by the locator for the page at the given index.
            """
            texts = await self.locators[page_index].all_text_contents()
            return {"status": "ok", "texts": texts}

        async def title(limit: int, page_index: int = 0) -> dict:
            """
            Get the title of the current page.
            """
            title = await self.pages[page_index].title()
            return {"status": "ok", "title": title}

        return [
            Tool(bring_to_front),
            Tool(click),
            Tool(click_nth),
            Tool(current_iso8601_utc_time),
            Tool(enter),
            Tool(fill),
            Tool(fill_active),
            Tool(filter),
            Tool(goto),
            Tool(locate),
            Tool(locate_by_css),
            Tool(locate_by_xpath),
            Tool(locate_by_role),
            Tool(new_tab),
            Tool(search),
            Tool(snapshot_full),
            Tool(snapshot_main),
            Tool(text_contents),
            Tool(title)
        ]

    @staticmethod
    async def start(browser):
        context = await browser.new_context()
        return BrowserSession(browser, context)


async def main(node):
    browser = await Browser.start("ws://localhost:3000/browser")
    browser_session = await BrowserSession.start(browser)
    tools = browser_session.tools()

    await Agent.start(
        node=node,
        name="jack",
        model=Model("nova-pro-v1"),
        tools=tools,
        instructions="""
            You are a web browsing agent.
            You are given a set of tools to drive a web browser and
            to interact with web pages.

            Your primary means of getting information from a page is
            through the snapshotss. When finding information on a page
            prefer snapshot_main and then snapshow_full over locators. However
            because snapshots are long, do not put snapshots directly in a response.

            For any search request, attempt fill_active then enter before resorting
            to locators, fill, and clicks.

            For any request you should first break the request down into a series
            of steps that you can accomplish with the tools you have available.
        """
    )


Node.start(main, llm_debug=True)
