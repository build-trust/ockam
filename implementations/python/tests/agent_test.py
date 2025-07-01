from ockam import Agent, Model, Node, Tool, McpTool, McpServer, McpClient, HttpServer

import pytest
import textwrap

from ockam.nodes.message import MessageConverter


def test_simple_node():
    Node.start(main_simple_node, wait_until_interrupted=False)


async def main_simple_node(node):
    await node.start_worker("echoer", Echoer())
    message = "hello"
    reply = await node.send_and_receive("echoer", "hello")
    assert reply == message


# def test_agent():
#     Node.start(main_agent, wait_until_interrupted=False, http_server=HttpServer(listen_address="127.0.0.1:0"))


async def main_agent(node):
    agent = await Agent.start(
        node=node,
        name="history-teacher",
        instructions="You are an assistant who is an expert in history.",
        model=Model(name="ollama_chat/llama3.2"),
    )
    reply = await agent.send("Who was Gandhi?")
    evaluator = await Agent.start(
        node=node,
        name="history-article-evaluator",
        instructions=textwrap.dedent("""You are an assistant who is an expert at history.

        You will be given an article and a question about the article,
        Reply to the question with only YES or NO
        Don't say anything else.
        Don't put any punctuations in your reply.
        """),
        model=Model(name="ollama_chat/llama3.2"),
    )
    evaluation = await evaluator.send("Article:\n" + reply[0].content.text + "\n\n\nQuestion: Is this article about Gandhi?")
    converter = MessageConverter(node)
    assert {"role": "assistant", "content": "YES", "tool_calls": []} == converter.message_to_dict(evaluation[0])


# def test_agent_can_call_agents_via_mcp():
#     Node.start(
#         main_agent_can_call_agents_via_mcp,
#         mcp_server=McpServer(listen_address="127.0.0.1:8001"),
#         mcp_clients=[
#             McpClient(name="localhost", address="http://127.0.0.1:8001/sse"),
#         ],
#         wait_until_interrupted=False,
#         http_server=HttpServer(listen_address="127.0.0.1:0")
#     )


async def main_agent_can_call_agents_via_mcp(node):
    await Agent.start(
        node=node,
        name="random-generator",
        instructions="Always answer with: '163728' and nothing more.",
        model=Model(name="ollama_chat/llama3.2", temperature=0.0),
        exposed_as="Random number generator",
    )

    agent = await Agent.start(
        node=node,
        name="Assistant",
        instructions="You are an assistant that only calls other assistants via tools and responds with their answer.",
        model=Model(name="ollama_chat/llama3.2", temperature=0.0),
        tools=[McpTool("localhost", "random-generator")],
    )

    reply = await agent.send("Give me a random number")
    assert "163728" in reply[0].content.text


# def test_agent_can_call_tools():
#     Node.start(main_agent_can_call_tools, wait_until_interrupted=False, http_server=HttpServer(listen_address="127.0.0.1:0"))


async def main_agent_can_call_tools(node):
    tool_tracker = ToolTracker()
    agent = await Agent.start(
        node=node,
        name="Calculator",
        instructions="You are an assistant that can multiply or divide two numbers.",
        model=Model(name="ollama_chat/llama3.2"),
        tools=[Tool(tool_tracker.divide), Tool(tool_tracker.multiply)],
    )
    reply = await agent.send("What is 56 divided by 27?")
    assert tool_tracker.divide_called
    reply = await agent.send("What is 214 multiplied by 63?")
    assert tool_tracker.multiply_called


class Echoer:
    async def handle_message(self, context, message):
        await context.reply(message)


class ToolTracker:
    def __init__(self):
        self.multiply_called = False
        self.divide_called = False

    def divide(self, a: float, b: float) -> float:
        self.divide_called = True
        a = float(a)
        b = float(b)
        return a / b

    def multiply(self, a: float, b: float) -> float:
        self.multiply_called = True
        a = float(a)
        b = float(b)
        return a * b
