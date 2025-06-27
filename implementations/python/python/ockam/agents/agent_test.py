from copy import deepcopy
from typing import List

import pytest

from .agent import Agent
from .. import Node, Tool, CoTPlanner, HttpServer
from ..nodes.message import Message, ConversationRole, Phase, ToolCall, FunctionToolCall


async def test_agent_name():
    with pytest.raises(ValueError):
        await Agent.start(name="!123-abc~", exposed_as="henry", node=None, instructions=None)


class Mock:
    def __init__(self, **kwargs):
        for key, value in kwargs.items():
            setattr(self, key, value)


class ModelSimulator:
    provided_messages: List[dict]
    max_input_tokens: int

    def __init__(self, messages: List[dict], max_input_tokens=None):
        self.provided_messages = messages or []
        self.max_input_tokens = max_input_tokens
        self.original_name = "MockModel"
        self.tool_call_counter = 0

    async def _complete_chat_streaming(self, provided_message: dict):
        delta = Mock()
        delta.role = provided_message.get("role", "assistant")
        delta.content = ""

        for character in provided_message.get("reasoning_content", ""):
            delta.reasoning_content = str(character)
            delta.content = None
            delta.tool_calls = []
            yield Mock(choices=[Mock(delta=deepcopy(delta), finish_reason=None)])

        for character in provided_message.get("content", ""):
            delta.content = str(character)
            delta.reasoning_content = None
            delta.tool_calls = []
            yield Mock(choices=[Mock(delta=deepcopy(delta), finish_reason=None)])

        delta.content = ""

        for provided_tool_call in provided_message.get("tool_calls", []):
            function = Mock()
            function.name = provided_tool_call["name"]
            function.arguments = ""
            tool_call = Mock()
            tool_call.type = "function"
            tool_call.id = f"tool_call_{self.tool_call_counter}"
            tool_call.index = str(self.tool_call_counter)
            self.tool_call_counter += 1
            tool_call.function = function
            delta.tool_calls = [tool_call]

            yield Mock(choices=[Mock(delta=deepcopy(delta), finish_reason=None)])
            function.name = None
            tool_call.id = None
            tool_call.type = None

            for character in provided_tool_call.get("arguments", ""):
                function.arguments = str(character)
                yield Mock(choices=[Mock(delta=deepcopy(delta), finish_reason=None)])

        delta.content = None
        delta.reasoning_content = None
        delta.tool_calls = []
        yield Mock(choices=[Mock(delta=deepcopy(delta), finish_reason="stop")])

    async def _complete_chat(self, provided_message: dict):
        message = Mock()
        message.content = provided_message.get("content", "")
        message.reasoning_content = provided_message.get("reasoning_content", "")
        message.role = provided_message.get("role", "assistant")
        message.tool_calls = []
        for tool_call in provided_message.get("tool_calls", []):
            function = Mock()
            function.name = tool_call["name"]
            function.arguments = tool_call["arguments"]
            tool_call = Mock()
            tool_call.id = f"tool_call_{len(message.tool_calls)}"
            tool_call.function = function
            message.tool_calls.append(tool_call)
        choice = Mock()
        choice.message = message

        instance = Mock()
        instance.choices = [choice]
        return instance

    async def complete_chat(self, messages: List[Message], stream: bool, **kwargs):
        print(f"MockModel called with messages: {messages}")

        if not self.provided_messages:
            raise ValueError("MockModel has no more messages to return")

        provided_message = self.provided_messages.pop(0)
        if stream:
            return self._complete_chat_streaming(provided_message)
        else:
            return await self._complete_chat(provided_message)

    async def embeddings(self, text, **kwargs):
        return [[0.1, 0.2, 0.3]] * len(text)


def weather_tool(argument: str):
    """
    Mock tool function that simulates a tool call.
    """
    return "sunny"


def test_full_flow():
    Node.start(_test_full_flow, wait_until_interrupted=False, http_server=HttpServer(listen_address="127.0.0.1:0"))


async def _test_full_flow(node):
    planner_model = ModelSimulator(
        [
            {
                "role": "assistant",
                "reasoning_content": "I'm thinking a lot...",
                "content": "The plan is to query the weather tool!",
            }
        ]
    )

    agent_model = ModelSimulator(
        [
            {"role": "assistant", "tool_calls": [{"name": "weather_tool", "arguments": '{"argument": "Paris"}'}]},
            {
                "role": "assistant",
                "reasoning_content": "I'm thinking that the result is sunny, so I should answer sunny...",
                "content": "The weather in Paris is sunny!",
            },
        ]
    )

    agent = await Agent.start(
        node=node,
        instructions="You are an agent that will test the full flow of an agent!",
        planner=CoTPlanner(model=planner_model),
        tools=[Tool(weather_tool)],
        model=agent_model,
    )

    messages = await agent.send("What is the weather in Paris?")
    assert len(messages) == 6

    assert messages[0].role == ConversationRole.USER
    assert messages[0].content == "I'm thinking a lot..."
    assert messages[0].phase == Phase.PLANNING
    assert messages[0].thinking

    assert messages[1].role == ConversationRole.USER
    assert messages[1].phase == Phase.PLANNING
    assert messages[1].content == "The plan is to query the weather tool!"
    assert messages[1].thinking is False

    assert messages[2].role == ConversationRole.ASSISTANT
    assert messages[2].phase == Phase.EXECUTING
    assert messages[2].content == ""
    assert messages[2].tool_calls == [
        ToolCall(
            id="tool_call_0",
            function=FunctionToolCall(name="weather_tool", arguments='{"argument": "Paris"}'),
            type="function",
        )
    ]

    assert messages[3].role == ConversationRole.TOOL
    assert messages[3].phase == Phase.EXECUTING
    assert messages[3].name == "weather_tool"
    assert messages[3].content == "sunny"
    assert messages[3].tool_call_id == "tool_call_0"

    assert messages[4].role == ConversationRole.ASSISTANT
    assert messages[4].phase == Phase.EXECUTING
    assert messages[4].content == "I'm thinking that the result is sunny, so I should answer sunny..."
    assert messages[4].thinking

    assert messages[5].role == ConversationRole.ASSISTANT
    assert messages[5].phase == Phase.EXECUTING
    assert messages[5].content == "The weather in Paris is sunny!"
    assert messages[5].thinking is False


def test_full_flow_streaming():
    Node.start(
        _test_full_flow_streaming, wait_until_interrupted=False, http_server=HttpServer(listen_address="127.0.0.1:0")
    )


async def _test_full_flow_streaming(node):
    planner_model = ModelSimulator(
        [
            {
                "role": "assistant",
                "reasoning_content": "I'm thinking a lot...",
                "content": "The plan is to query the weather tool!",
            }
        ]
    )

    agent_model = ModelSimulator(
        [
            {"role": "assistant", "tool_calls": [{"name": "weather_tool", "arguments": '{"argument": "Paris"}'}]},
            {
                "role": "assistant",
                "reasoning_content": "I'm thinking that the result is sunny, so I should answer sunny...",
                "content": "The weather in Paris is sunny!",
            },
        ]
    )

    agent = await Agent.start(
        node=node,
        instructions="You are an agent that will test the full flow of an agent!",
        planner=CoTPlanner(model=planner_model),
        tools=[Tool(weather_tool)],
        model=agent_model,
    )

    chunks = []
    async for chunk in agent.send_stream("What is the weather in Paris?"):
        chunks.append(chunk)

    # assemble the messages back
    messages = []

    # I'm thinking a lot...
    message = chunks.pop(0).snippet.messages[0]
    for _ in range(20):
        message.content += chunks.pop(0).snippet.messages[0].content
    messages.append(message)

    # The plan is to query the weather tool!
    message = chunks.pop(0).snippet.messages[0]
    for _ in range(37):
        message.content += chunks.pop(0).snippet.messages[0].content
    messages.append(message)

    # tool call - exposed as a single message
    messages.append(chunks.pop(0).snippet.messages[0])

    # tool response - exposed as a single message
    messages.append(chunks.pop(0).snippet.messages[0])

    # I'm thinking that the result is sunny, so I should answer sunny...
    message = chunks.pop(0).snippet.messages[0]
    for _ in range(65):
        message.content += chunks.pop(0).snippet.messages[0].content
    messages.append(message)

    # The weather in Paris is sunny!
    message = chunks.pop(0).snippet.messages[0]
    for _ in range(29):
        message.content += chunks.pop(0).snippet.messages[0].content
    messages.append(message)

    # last finished message
    last_chunk = chunks.pop(0)
    assert last_chunk.snippet.messages == []
    assert last_chunk.finished

    assert len(messages) == 6

    assert messages[0].role == ConversationRole.USER
    assert messages[0].content == "I'm thinking a lot..."
    assert messages[0].phase == Phase.PLANNING
    assert messages[0].thinking

    assert messages[1].role == ConversationRole.USER
    assert messages[1].phase == Phase.PLANNING
    assert messages[1].content == "The plan is to query the weather tool!"
    assert messages[1].thinking is False

    assert messages[2].role == ConversationRole.ASSISTANT
    assert messages[2].phase == Phase.EXECUTING
    assert messages[2].content == ""
    assert messages[2].tool_calls == [
        ToolCall(
            id="tool_call_0",
            function=FunctionToolCall(name="weather_tool", arguments='{"argument": "Paris"}'),
            type="function",
        )
    ]

    assert messages[3].role == ConversationRole.TOOL
    assert messages[3].phase == Phase.EXECUTING
    assert messages[3].name == "weather_tool"
    assert messages[3].content == "sunny"
    assert messages[3].tool_call_id == "tool_call_0"

    assert messages[4].role == ConversationRole.ASSISTANT
    assert messages[4].phase == Phase.EXECUTING
    assert messages[4].content == "I'm thinking that the result is sunny, so I should answer sunny..."
    assert messages[4].thinking

    assert messages[5].role == ConversationRole.ASSISTANT
    assert messages[5].phase == Phase.EXECUTING
    assert messages[5].content == "The weather in Paris is sunny!"
    assert messages[5].thinking is False
