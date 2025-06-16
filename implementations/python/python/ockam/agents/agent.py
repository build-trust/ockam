import json
import traceback
from typing import Optional, AsyncGenerator, Tuple, Dict, List
from enum import Enum

import secrets

from .conversation_response import ConversationResponse
from ..nodes import RemoteNode, LocalNodeProtocol
from ..planning import Planner
from ..tools.protocol import InvokableTool
from ..knowledge import SearchResults, KnowledgeProvider
from ..memory import Memory
from ..models import Model
from ..nodes import NodeProtocol
from ..nodes.message import (
    ConversationSnippet,
    StreamedConversationSnippet,
    ConversationRole,
    Error,
    GetIdentifierRequest,
    GetIdentifierResponse,
    MessageConverter,
    AgentReference,
    ConversationMessage,
    AssistantMessage,
    ToolCall,
    GetConversationsRequest,
    GetConversationsResponse,
    Phase,
    ToolCallResponseMessage,
)
from .names import validate_name

from ..ockam_in_rust_for_python import warn


class AgentState(Enum):
    """
    States for the agent conversation state machine.

    - INIT: Initial state where we set up the conversation and initialize planning if available
    - PLANNING: Create the next step of the plan
    - MODEL_CALLING: Call the model to generate a response
    - TOOL_CALLING: Execute tool calls from the model response
    - FINISHED: End of the conversation

    ┌─────────┐
    │         │
    │  INIT   │
    │         │
    └────┬────┘
         │
         ├─────────────────────────┐
         │                         │
         │ [planner available]     │ [planner not available]
         ▼                         ▼
    ┌─────────┐               ┌──────────┐
    │         │               │          │
    │PLANNING │◀──[next step]─┤ MODEL    │◀─────┐
    │         ├──[execute]───▶│ CALLING  │      │
    └────┬────┘               └┬────┬────┘      │
         │                     │    │           │
         │                     │    │           │
         │                     │    │           │
         │                     │    │           │
         │                     │    │           │
         │                     │    ▼           │
         │                     │┌────────┐      │
         │                     ││        │      │
         │                     ││  TOOL  │──────┘
         │ [plan completed]    ││CALLING │
         │                     │└────────┘
         │                     │
         ▼                     │
    ┌─────────┐                │
    │         │                │
    │FINISHED │◀───────────────┘
    │         │
    └─────────┘
    """

    INIT = "init"
    PLANNING = "planning"
    MODEL_CALLING = "model_calling"
    TOOL_CALLING = "tool_calling"
    FINISHED = "finished"


class AgentStateMachine:
    """
    State machine for managing agent conversation flow.

    This class implements a state machine that manages the flow of an agent conversation.
    It handles the transitions between different states and executes the appropriate actions
    for each state. The state machine is designed to be used by the `handle__conversation_snippet`
    method of the `Agent` class.
    """

    def __init__(self, agent, scope, conversation, stream, response, contextual_knowledge=None):
        self.agent = agent
        self.scope = scope
        self.conversation = conversation
        self.stream = stream
        self.streaming_response = response
        self.contextual_knowledge = contextual_knowledge
        self.state = AgentState.INIT
        self.plan = None
        self.iteration = 0
        self.tool_calls = []
        self.whole_response = ConversationSnippet(scope, conversation, [])

    async def transition(self):
        """Execute the current state and determine the next state."""

        match self.state:
            case AgentState.INIT:
                if self.agent.planner is not None:
                    self.state = AgentState.PLANNING
                else:
                    self.state = AgentState.MODEL_CALLING
            case AgentState.PLANNING:
                async for result in self._handle_planning_state():
                    yield result
            case AgentState.MODEL_CALLING:
                async for result in self._handle_model_calling_state():
                    yield result
            case AgentState.TOOL_CALLING:
                async for result in self._handle_tool_calling_state():
                    yield result

    async def _handle_planning_state(self):
        """Handle the PLANNING state: Execute the next steps from the plan."""

        plan_completed = True
        next_steps = self.plan.next_step(
            await self.agent.get_messages_only(self.conversation, self.scope), self.contextual_knowledge
        )

        async for next_step in next_steps:
            if next_step is None:
                break
            plan_completed = False
            self.contextual_knowledge = await self.agent.add_knowledge_search(next_step.content)
            if next_step.phase == Phase.PLANNING:
                await self.agent.remember(self.scope, self.conversation, next_step)
            if self.stream:
                yield self.streaming_response.make_snippet(next_step)
            else:
                self.whole_response.messages.append(next_step)

        if plan_completed:
            # We executed the whole plan
            self.state = AgentState.FINISHED
        else:
            # The model will execute the next plan step
            self.state = AgentState.MODEL_CALLING

    async def _handle_model_calling_state(self):
        """Handle the MODEL_CALLING state: Call the model to generate a response."""

        self.iteration += 1
        if self.iteration == self.agent.maximum_iterations:
            yield Error(str(RuntimeError("Reached maximum_iterations")))
            self.state = AgentState.FINISHED

        self.tool_calls = []
        async for error, finished, model_response in self.agent.complete_chat(
            self.scope, self.conversation, self.contextual_knowledge, stream=self.stream
        ):
            if error:
                yield self.streaming_response.make_snippet(error)
                self.state = AgentState.FINISHED
                return

            # Remember the model response
            await self.agent.remember(self.scope, self.conversation, model_response)

            if len(model_response.tool_calls) > 0:
                # Record the event of the tool being called
                if self.stream:
                    yield self.streaming_response.make_snippet(model_response)
                else:
                    self.whole_response.messages.append(model_response)
                # Store tool calls for later processing
                self.tool_calls.extend(model_response.tool_calls)
                self.state = AgentState.TOOL_CALLING
            elif finished or not self.stream:
                # either we are streaming and we finished, or we are expecting a single response
                if self.stream:
                    yield self.streaming_response.make_snippet(model_response)
                else:
                    self.whole_response.messages.append(model_response)

                if self.state == AgentState.TOOL_CALLING:
                    return
                else:
                    if self.agent.planner is None:
                        self.state = AgentState.FINISHED
                    else:
                        self.state = AgentState.PLANNING
                    return
            else:
                # Handle streamed response chunk
                yield self.streaming_response.make_snippet(model_response)

    async def _handle_tool_calling_state(self):
        """Handle the TOOL_CALLING state: Process tool calls from the model response."""

        for tool_call in self.tool_calls:
            error, tool_call_response = await self.agent.call_tool(tool_call)
            if error:
                warn(f"MCP call failed: {error}")

            # the tool_call_response contains the error message if the tool call failed
            await self.agent.remember(self.scope, self.conversation, tool_call_response)
            if self.stream:
                yield self.streaming_response.make_snippet(tool_call_response)
            else:
                self.whole_response.messages.append(tool_call_response)

        # after processing tool calls, we need another model call
        self.state = AgentState.MODEL_CALLING

    async def _handle_finished_state(self):
        """Handle the FINISHED state: End the conversation and return the final response."""

        if self.stream:
            yield self.streaming_response.make_finished_snippet()
        else:
            yield self.whole_response

    async def initialize_plan(self, messages):
        """Initialize the plan if a planner is available and remember messages."""

        if self.agent.planner is None:
            for message in messages:
                await self.agent.remember(self.scope, self.conversation, message)
        else:
            # When a planner is provided, the input messages will be handled solely by the planner
            self.plan = await self.agent.planner.plan(messages, self.contextual_knowledge, self.stream)

    async def run(self):
        """Run the state machine until completion."""

        while self.state != AgentState.FINISHED:
            async for result in self.transition():
                yield result

        async for result in self._handle_finished_state():
            yield result


class Agent:
    _logger = None
    tools: Dict[str, InvokableTool]

    @classmethod
    def logger(cls):
        if cls._logger:
            return cls._logger
        else:
            from ..logging.logging import get_logger

            cls._logger = get_logger("agent")
            return cls._logger

    def __init__(
        self,
        node: LocalNodeProtocol,
        name: str,
        instructions: str,
        model: Model,
        tool_specs: List[dict],
        tools: Dict[str, InvokableTool],
        planner: Planner,
        memory: Memory,
        knowledge: KnowledgeProvider,
        max_knowledge_size: int,
    ):
        self.logger = Agent.logger()
        self.logger.info(f"start agent '{name}'")
        self.node = node

        self.tools = tools
        self.tool_specs = tool_specs

        self.name = name
        self.model = model

        self.memory = memory
        memory.set_instructions(system_message(instructions))

        self.planner = planner
        self.maximum_iterations = 14

        self.knowledge = knowledge
        self.max_knowledge_size = max_knowledge_size
        self.search_results = SearchResults()

        self.converter = MessageConverter.create(node)

    async def handle_message(self, context, message):
        try:
            self.logger.debug(f"agent '{self.name}' received: {message}")

            message = self.converter.message_from_json(message)
            handlers = {
                ConversationSnippet: self.handle__conversation_snippet,
                StreamedConversationSnippet: self.handle__conversation_snippet,
                GetIdentifierRequest: self.handle__get_identifier_request,
                GetConversationsRequest: self.handle__get_conversations_request,
            }

            handler = handlers.get(type(message))
            if type(message) is ConversationSnippet or type(message) is StreamedConversationSnippet:
                async for reply in handler(message):
                    if reply is not None:
                        context.reply(self.converter.message_to_json(reply))
            else:
                if handler is not None:
                    reply = await handler(message)
                else:
                    self.logger.error(f"unexpected message: {message}")
                    reply = Error(f"Unexpected Message: {message}")

                if reply is not None:
                    context.reply(self.converter.message_to_json(reply))
        except Exception as e:
            traceback.print_exc()
            error = Error(str(e))
            context.reply(self.converter.message_to_json(error))

    async def handle__get_identifier_request(self, message: GetIdentifierRequest) -> GetIdentifierResponse:
        name_snake_case = self.name.lower().replace(" ", "_")
        node_identifier = await self.node.identifier()
        agent_identifier = f"{node_identifier}/{name_snake_case}"
        return GetIdentifierResponse(message.scope, message.conversation, agent_identifier)

    async def handle__get_conversations_request(self, message: GetConversationsRequest) -> GetConversationsResponse:
        return GetConversationsResponse(self.memory.get_messages_only(message.scope, message.conversation))

    async def handle__conversation_snippet(
        self, snippet: StreamedConversationSnippet | ConversationSnippet
    ) -> AsyncGenerator[StreamedConversationSnippet | ConversationSnippet | Error, None]:
        stream = type(snippet) is StreamedConversationSnippet
        if stream:
            snippet = snippet.snippet

        scope = snippet.scope
        if not scope:
            scope = secrets.token_hex(16)

        conversation = snippet.conversation
        if not conversation:
            conversation = secrets.token_hex(16)

        response = ConversationResponse(scope, conversation, stream)
        messages = snippet.messages

        query = ""
        for message in messages:
            if message.role == ConversationRole.USER:
                query = message.content

        contextual_knowledge = None
        if self.knowledge is not None:
            contextual_knowledge = await self.add_knowledge_search(query)

        # Create and initialize the state machine
        state_machine = AgentStateMachine(
            agent=self,
            scope=scope,
            conversation=conversation,
            stream=stream,
            response=response,
            contextual_knowledge=contextual_knowledge,
        )

        # Initialize the plan if needed
        await state_machine.initialize_plan(messages)

        # Run the state machine
        async for result in state_machine.run():
            yield result

    async def get_messages_only(self, conversation, scope) -> list[ConversationMessage]:
        messages: list[dict] = self.memory.get_messages_only(scope, conversation)
        return [self.converter.conversation_message_from_dict(m) for m in messages]

    async def add_knowledge_search(self, query: str) -> Optional[str]:
        """
        Search the knowledge base for relevant information and adds it to the search results.
        """
        # Check if knowledge is None or query is empty
        if self.knowledge is None or not query or len(query) == 0:
            return None

        hits = await self.knowledge.search(query)
        self.search_results.add(hits)
        while True:
            view = self.search_results.view()
            if len(view) > 0:
                contextual_knowledge = ""
                for document_name, text_pieces in view.items():
                    contextual_knowledge += f"Document name: {document_name}\n"
                    for text_piece in text_pieces:
                        contextual_knowledge += f"- {text_piece}\n"
                    contextual_knowledge += "\n"
                if len(contextual_knowledge) <= self.max_knowledge_size:
                    break
            else:
                contextual_knowledge = None
                break
            self.search_results.reduce_size()
        return contextual_knowledge

    async def remember(self, scope: str, conversation: str, message: ConversationMessage):
        if not isinstance(message, dict):
            message = self.converter.message_to_dict(message)
        self.memory.add_message(scope, conversation, message)

    async def message_history(self, scope: str, conversation: str) -> list[dict]:
        return self.memory.get_messages(scope, conversation)

    async def complete_chat(
        self, scope, conversation, contextual_knowledge, stream: bool = False
    ) -> AsyncGenerator[Tuple[Optional[Exception], bool, AssistantMessage], None]:
        message_history = await self.message_history(scope, conversation)

        if contextual_knowledge is not None:
            message_history = [
                {
                    "role": "system",
                    "content": "The following knowledge could be useful to answer properly:\n" + contextual_knowledge,
                }
            ] + message_history

        response = await self.model.complete_chat(tools=self.tool_specs, messages=message_history, stream=stream)
        if stream:
            tool_calls: Dict[int, ToolCall] = {}
            async for chunk in response:
                finished = chunk.choices[0].finish_reason is not None
                delta = chunk.choices[0].delta

                if delta.tool_calls:
                    for tool_call in delta.tool_calls:
                        if tool_call.index in tool_calls:
                            tool_calls[tool_call.index].function.arguments += tool_call.function.arguments
                        else:
                            tool_calls[tool_call.index] = tool_call
                    delta.tool_calls = None

                # return all the tool calls at the end of the response
                if finished:
                    if len(tool_calls) > 0:
                        response = AssistantMessage()
                        response.tool_calls = list(tool_calls.values())
                        yield None, False, response

                if finished or (delta.content is not None and len(delta.content) > 0):
                    yield await self.send_response(chunk)
        else:
            yield await self.send_response(response)

    async def send_response(self, response) -> (Error | None, int, bool, AssistantMessage):
        """
        This function converts the model response into an AssistantMessage.
        When streaming is used the function also returns a boolean indicating if this is the last part of the response
        """
        if response is None or not hasattr(response, "choices") or not response.choices:
            e = ValueError(f"The model returned a response with an unexpected structure - {response}")
            error = Error(str(e))
            return error, None

        choice = response.choices[0]
        finished = False
        if hasattr(choice, "delta"):
            delta = choice.delta
            # sometimes the role is not set
            if delta.role is None:
                role = "assistant"
            else:
                role = delta.role

            if choice.finish_reason is not None:
                self.logger.debug(f"finished streaming reply with reason {choice.get('finish_reason', 'unknown')}")
                finished = True
                response = {"role": role}
            else:
                response = {"role": role, "content": delta.content}
            message = delta
        else:
            message = choice.message
            response = {"role": message.role, "content": message.content}

        if message.tool_calls:
            response["tool_calls"] = []
            for tool_call in message.tool_calls:
                id = tool_call.id
                name = tool_call.function.name
                args = tool_call.function.arguments
                response["tool_calls"].append({"id": id, "function": {"name": name, "arguments": args}})

        return None, finished, self.converter.conversation_message_from_dict(response)

    async def call_tool(self, tool_call: ToolCall) -> Tuple[Optional[Error], ToolCallResponseMessage]:
        id = tool_call.id
        name = tool_call.function.name
        args = tool_call.function.arguments

        tool = self.tools.get(name)
        if not tool:
            error_response = f"Tool '{name}' not found."
            error = Error(error_response)
            return error, ToolCallResponseMessage(id, name, error_response)

        try:
            response = await tool.invoke(args)
            return None, ToolCallResponseMessage(id, name, response)
        except BaseException as e:
            error_response = f"Tool '{name}' invocation failed: {str(e)}"
            return Error(error_response), ToolCallResponseMessage(id, name, error_response)

    @staticmethod
    async def start(
        node: NodeProtocol,
        instructions: str,
        name: Optional[str] = None,
        model: Model = None,
        tools: Optional[List[InvokableTool]] = None,
        planner: Planner = None,
        exposed_as: Optional[str] = None,
        knowledge: Optional[KnowledgeProvider] = None,
        max_knowledge_size: int = 4096,
    ):
        if name is None:
            name = secrets.token_hex(12)

        validate_name(name)

        if model is None:
            model = Model(name="llama3.2")

        match node:
            case node if isinstance(node, LocalNodeProtocol):
                await Agent.start_agent_impl(
                    node, instructions, name, model, tools, planner, exposed_as, knowledge, max_knowledge_size
                )
            case node if isinstance(node, RemoteNode):
                await node.start_agent(
                    instructions, name, model, tools, planner, exposed_as, knowledge, max_knowledge_size
                )
                Agent.logger().info(f"Successfully started agent {name} on a remote node")
            case _:
                raise ValueError("Node must be either a LocalNodeProtocol or a RemoteNode")

        return AgentReference(name, node, exposed_as)

    @staticmethod
    async def stop(node, name):
        await node.stop_worker(name)

    @staticmethod
    async def start_many(
        node: NodeProtocol,
        instructions: str,
        number_of_agents: int,
        model: Model = None,
        tools: Optional[list] = None,
        planner=None,
        knowledge: Optional[KnowledgeProvider] = None,
        max_knowledge_size: int = 4096,
    ):
        if model is None:
            model = Model(name="llama3.2")

        agents = []

        match node:
            case node if isinstance(node, LocalNodeProtocol):
                for i in range(number_of_agents):
                    name = secrets.token_hex(12)
                    await Agent.start_agent_impl(
                        node, instructions, name, model, tools, planner, None, knowledge, max_knowledge_size
                    )
                    agents.append(AgentReference(name, node))
            case node if isinstance(node, RemoteNode):
                names = await node.start_agents(
                    instructions, number_of_agents, model, tools, planner, knowledge, max_knowledge_size
                )

                for name in names:
                    agents.append(AgentReference(name, node))

                Agent.logger().info("Successfully started agents on a remote node")
            case _:
                raise ValueError("Node must be either a LocalNodeProtocol or a RemoteNode")

        return agents

    @staticmethod
    async def start_agent_impl(
        node: LocalNodeProtocol,
        instructions: str,
        name: Optional[str],
        model: Model,
        tools: Optional[List[InvokableTool]],
        planner: Planner,
        exposed_as: Optional[str],
        knowledge: Optional[KnowledgeProvider],
        max_knowledge_size: int,
    ):
        tools_specs, tools = await prepare_tools(node, tools)
        # the memory is shared between all the agent workers
        memory = Memory()

        def agent_creator():
            return Agent(
                node, name, instructions, model, tools_specs, tools, planner, memory, knowledge, max_knowledge_size
            )

        await node.start_spawner(name, agent_creator, key_extractor, None, exposed_as)

        Agent.logger().debug(f"successfully started agent {name}")


def key_extractor(message):
    message = json.loads(message)
    scope = message.get("scope")
    conversation = message.get("conversation")
    if conversation:
        if scope:
            return f"{scope}/{conversation}"
    return None


async def prepare_tools(node, tools) -> Tuple[List[dict], Dict[str, InvokableTool]]:
    specs = []
    prepared = {}
    if tools is None:
        tools = []

    for tool in tools:
        prepared[tool.name] = tool
        tool.node = node
        spec = await tool.spec()
        specs.append(spec)

    return specs, prepared


def system_message(message: str) -> dict:
    return {"role": "system", "content": message}
