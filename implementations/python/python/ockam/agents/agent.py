import json
import traceback

from typing import List
from enum import Enum

from copy import deepcopy
from typing import Optional, AsyncGenerator, Tuple, Dict

import secrets
import copy

from .conversation_response import ConversationResponse
from ..tools.protocol import InvokableTool
from .agent_memory_knowledge import AgentMemoryKnowledge
from ..knowledge.noop import NoopKnowledge
from ..nodes import RemoteNode, LocalNodeProtocol
from ..logging.logging import InfoContext, DebugContext
from ..planning import Planner
from ..knowledge import KnowledgeProvider
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


class AgentStateMachine(InfoContext):
    """
    State machine for managing agent conversation flow.

    This class implements a state machine that manages the flow of an agent conversation.
    It handles the transitions between different states and executes the appropriate actions
    for each state. The state machine is designed to be used by the `handle__conversation_snippet`
    method of the `Agent` class.
    """

    def __init__(self, agent, scope, conversation, stream, response, contextual_knowledge=None):
        self.agent = agent
        self.logger = agent.class_logger()
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
                    self.logger.info(f"The agent '{self.agent.name}' is in Planning mode")
                    self.state = AgentState.PLANNING
                else:
                    self.logger.info(f"The agent '{self.agent.name}' is in Model Calling mode")
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

        steps = []
        async for next_step in next_steps:
            if next_step is None:
                break
            plan_completed = False
            steps.append(next_step)
            if next_step.phase == Phase.PLANNING:
                await self.agent.remember(self.scope, self.conversation, next_step)
            if self.stream:
                yield self.streaming_response.make_snippet(next_step)
            else:
                self.whole_response.messages.append(next_step)

        if len(steps) > 0:
            self.logger.debug(
                f"The agent '{self.agent.name}' plan has {len(steps)} messages to execute\n{[s.content for s in steps]}"
            )
        if plan_completed:
            # We executed the whole plan
            self.logger.info(f"The agent '{self.agent.name}' plan is now completed")
            self.state = AgentState.FINISHED
        else:
            # The model will execute the next plan step
            self.logger.info(f"The agent '{self.agent.name}' is executing the next step of the plan")
            self.logger.info(f"The agent '{self.agent.name}' is in now in Model Calling mode")
            self.state = AgentState.MODEL_CALLING

    async def _handle_model_calling_state(self):
        """Handle the MODEL_CALLING state: Call the model to generate a response."""

        self.iteration += 1
        if self.iteration == self.agent.maximum_iterations:
            yield Error(str(RuntimeError("Reached maximum_iterations")))
            self.logger.info(
                f"The agent '{self.agent.name}' execution is now finished due to maximum iterations reached ({self.agent.maximum_iterations})"
            )
            self.state = AgentState.FINISHED

        self.tool_calls = []
        async for err, finished, model_response in self.agent.complete_chat(
                self.scope, self.conversation, self.contextual_knowledge, stream=self.stream
        ):
            if err:
                yield err
                self.logger.info(f"The agent '{self.agent.name}' execution is now finished due to a model error: {err}")
                self.state = AgentState.FINISHED
                return

            # Remember the model response
            if not self.stream:
                self.logger.debug(
                    f"The agent '{self.agent.name}' is remembering the model response for scope '{self.scope}' and conversation '{self.conversation}'"
                )
            await self.agent.remember(self.scope, self.conversation, model_response)

            if len(model_response.tool_calls) > 0:
                if len(model_response.tool_calls) == 1:
                    self.logger.info(
                        f"The model response returned to agent '{self.agent.name}' contains one tool call, processing it now."
                    )
                else:
                    self.logger.info(
                        f"The model response returned to agent '{self.agent.name}' contains {len(model_response.tool_calls)} tool calls, processing them now."
                    )
                # Record the event of the tool being called
                if self.stream:
                    yield self.streaming_response.make_snippet(model_response)
                else:
                    self.whole_response.messages.append(model_response)
                # Store tool calls for later processing
                self.tool_calls.extend(model_response.tool_calls)
                self.logger.info(f"The agent '{self.agent.name}' is in now in Tool Calling mode")
                self.state = AgentState.TOOL_CALLING
            elif finished:
                # either we are streaming and we finished, or we are expecting a single response
                if self.stream:
                    if model_response.content or model_response.tool_calls:
                        yield self.streaming_response.make_snippet(model_response)
                else:
                    self.whole_response.messages.append(model_response)

                if self.state == AgentState.TOOL_CALLING:
                    return
                else:
                    if self.agent.planner is None:
                        self.logger.info(f"The agent '{self.agent.name}' has now finished its execution")
                        self.state = AgentState.FINISHED
                    else:
                        self.logger.info(f"The agent '{self.agent.name}' is going back to the Planning mode now")
                        self.state = AgentState.PLANNING
                    return
            else:
                if self.stream:
                    if model_response.content or model_response.tool_calls:
                        yield self.streaming_response.make_snippet(model_response)
                else:
                    self.whole_response.messages.append(model_response)

    async def _handle_tool_calling_state(self):
        """Handle the TOOL_CALLING state: Process tool calls from the model response."""

        for tool_call in self.tool_calls:
            with self.info(
                    f"The agent '{self.agent.name}' is calling the tool '{tool_call.function.name}' with arguments: '{tool_call.function.arguments}'",
                    f"The agent '{self.agent.name}' called the tool '{tool_call.function.name}' with arguments: '{tool_call.function.arguments}'",
            ):
                error, tool_call_response = await self.agent.call_tool(tool_call)
                if error:
                    self.logger.warning(
                        f"The agent {self.agent.name} called a tool '{tool_call.function.name}' with arguments: '{tool_call.function.arguments}', but the call failed: {error}"
                    )

                # the tool_call_response contains the error message if the tool call failed
                await self.agent.remember(self.scope, self.conversation, tool_call_response)
                if self.stream:
                    yield self.streaming_response.make_snippet(tool_call_response)
                else:
                    self.whole_response.messages.append(tool_call_response)

        # after processing tool calls, we need another model call
        self.logger.info(
            f"The agent '{self.agent.name}' has finished making tool calls. It is going back to the Model Calling mode"
        )
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
            self.logger.info(f"The agent '{self.agent.name}' is now planning the next steps")
            # When a planner is provided, the input messages will be handled solely by the planner
            self.plan = await self.agent.planner.plan(messages, self.contextual_knowledge, self.stream)

    async def run(self):
        """Run the state machine until completion."""

        while self.state != AgentState.FINISHED:
            async for result in self.transition():
                yield result

        async for result in self._handle_finished_state():
            yield result


class Agent(InfoContext, DebugContext):
    _logger = None
    tools: Dict[str, InvokableTool]
    memory_knowledge: Optional[AgentMemoryKnowledge]

    @classmethod
    def class_logger(cls):
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
            memory_model: Optional[Model],
            memory_embeddings_model: Optional[Model],
            tool_specs: List[dict],
            tools: Dict[str, InvokableTool],
            planner: Planner,
            memory: Memory,
            knowledge: KnowledgeProvider,
            maximum_iterations: int,
    ):
        self.logger = Agent.class_logger()
        with self.info(f"Starting agent '{name}'", f"Started agent '{name}'"):
            self.node = node

            self.tools = tools
            self.tool_specs = tool_specs

        self.name = name
        self.model = model
        self.memory_model = memory_model
        self.memory_embeddings_model = memory_embeddings_model
        self.memory_knowledge = None

        self.memory = memory
        memory.set_instructions(system_message(instructions))

        self.planner = planner
        self.maximum_iterations = maximum_iterations

        self.knowledge = knowledge

        self.converter = MessageConverter.create(node)

    async def handle_message(self, context, message):
        try:
            self.logger.debug(f"Agent '{self.name}' received: {message}")

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
                    self.logger.error(f"Unexpected message: {message}")
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
        for message in reversed(messages):
            if message.role == ConversationRole.USER:
                # TODO: Why we only use the last message to query knowledge?
                query = message.content
                break

        contextual_knowledge = await self.knowledge.search_knowledge(scope, conversation, query)

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

    async def remember(self, scope: str, conversation: str, message: ConversationMessage):
        # TODO: at some point memory becomes bigger than context window and we need to start pusing memories
        # into Knowledge
        # self.history_knowledge.add(messages=[model_response], user_id=scope)
        if not isinstance(message, dict):
            message = self.converter.message_to_dict(message)
        self.memory.add_message(scope, conversation, message)

    async def message_history(self, scope: str, conversation: str) -> list[dict]:
        return self.memory.get_messages(scope, conversation)

    async def determine_input_context(self, scope, conversation, contextual_knowledge):
        # Try to use full history and reduce it until it fits into the context window
        prompt_size = 1
        if contextual_knowledge:
            prompt_size += 1
            contextual_knowledge = [
                {
                    "role": "system",
                    "content": "The following knowledge could be useful to answer properly:\n" + contextual_knowledge,
                }
            ]
        else:
            contextual_knowledge = []

        # TODO: contextual_knowledge go after the initial prompt?
        message_history = contextual_knowledge + await self.message_history(scope, conversation)

        if not self.model.max_input_tokens:
            return message_history

        # FIXME: this should be fixed in the future for the case when model has max_input_tokens but we don't want
        #  to use memory in that case we should raise error only when we exceed max_input_tokens
        if not self.memory_model or not self.memory_embeddings_model:
            raise ValueError("Max input tokens exceeded and no memory model is available")

        prompts = deepcopy(message_history[:prompt_size])
        visible_message_history = deepcopy(message_history[prompt_size:])
        skipped_message_history = []

        query = None
        for message in reversed(message_history):
            if message.get("role", None) == "user":
                # TODO: Why we only use the last message to query knowledge?
                query = deepcopy(message.get("content", None))
                break

        iterations = 0
        max_iterations = int((len(visible_message_history) + 1) / 2) - 1

        while True:
            self.memory_knowledge = await AgentMemoryKnowledge.create(self.memory_model, self.memory_embeddings_model)
            for i in range(0, len(skipped_message_history), 2):
                self.logger.info(f"ADDING MESSAGES to knowledge: {skipped_message_history[i: i + 2]}")
                await self.memory_knowledge.add(
                    scope=scope, conversation=conversation, messages=skipped_message_history[i: i + 2]
                )

            # We should actually clear knowledge in this function each time...
            memory_knowledge = await self.memory_knowledge.search(scope, conversation, query)
            self.logger.info(f"RELATED MEMORY for query: {query}:\n{memory_knowledge}.")
            if memory_knowledge:
                # FIXME: Use tool role, maybe need to prepend tool call before that
                memory_knowledge = [
                    {
                        "role": "system",
                        "content": f"The following information from your past interactions with the user could be useful to answer properly:\n{memory_knowledge}",
                    }
                ]
            else:
                memory_knowledge = []

            self.logger.info(f"VISIBLE_MEMORY_HISTORY: {visible_message_history}")
            input_context = prompts + memory_knowledge + visible_message_history
            number_of_tokens = self.model.count_tokens(
                tools=self.tool_specs,
                messages=input_context,
            )

            self.logger.info(
                f"Skipped {iterations} turns out of {max_iterations} and now has {number_of_tokens} tokens. Input context: {input_context}"
            )

            if number_of_tokens <= self.model.max_input_tokens:
                return input_context

            # FIXME: Assuming turn is 2 messages is wrong, but works as initial implementation
            skipped_message_history.extend(deepcopy(visible_message_history[:2]))
            visible_message_history = deepcopy(visible_message_history[2:])

            if not visible_message_history:
                raise ValueError("Max input tokens exceeded and no message fits into the context window")

            iterations += 1

    async def complete_chat(
            self, scope, conversation, contextual_knowledge, stream: bool = False
    ) -> AsyncGenerator[Tuple[Optional[Exception], bool, AssistantMessage], None]:
        input_context = await self.determine_input_context(scope, conversation, contextual_knowledge)
        self.logger.info(
            f"Sending {len(input_context)} messages from agent '{self.name}' to model '{self.model.original_name}'"
        )
        response = await self.model.complete_chat(tools=self.tool_specs, messages=input_context, stream=stream)

        if not stream:
            messages = [choice.message for choice in response.choices]
            plural = "s" if len(messages) > 1 else ""
            self.logger.info(
                f"Received {len(messages)} message{plural} from model '{self.model.original_name}' for agent '{self.name}'"
            )

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

                async for err, finished, model_response in self.send_response(chunk):
                    yield err, finished, model_response
        else:
            async for err, finished, model_response in self.send_response(response):
                yield err, finished, model_response

    async def send_response(self, response) -> (Optional[Error], int, bool, AssistantMessage):
        """
        This function converts the model response into an AssistantMessage.
        When streaming is used the function also returns a boolean indicating if this is the last part of the response
        """
        if response is None or not hasattr(response, "choices") or not response.choices:
            e = ValueError(f"The model returned a response with an unexpected structure - {response}")
            error = Error(str(e))
            yield error, True, None

        choice = response.choices[0]
        if hasattr(choice, "delta"):
            finished = False
            delta = choice.delta
            # sometimes the role is not set
            if delta.role is None:
                role = "assistant"
            else:
                role = delta.role

            if choice.finish_reason is not None:
                self.logger.debug(f"Finished streaming reply with reason {choice.finish_reason or 'unknown'}")
                finished = True
                response = {"role": role}
            else:
                if hasattr(delta, "reasoning_content") and delta.reasoning_content:
                    response = {"role": role, "content": delta.reasoning_content, "thinking": True}
                    yield None, False, self.converter.conversation_message_from_dict(response)
                response = {"role": role, "content": delta.content}
            message = delta
        else:
            finished = True
            message = choice.message
            if hasattr(message, "reasoning_content") and message.reasoning_content:
                yield (
                    None,
                    False,
                    self.converter.conversation_message_from_dict(
                        {"role": message.role, "content": message.reasoning_content, "thinking": True}
                    ),
                )
                message.reasoning_content = ""

            response = {"role": message.role, "content": message.content}

        if message.tool_calls:
            response["tool_calls"] = []
            for tool_call in message.tool_calls:
                id = tool_call.id
                name = tool_call.function.name
                args = tool_call.function.arguments
                response["tool_calls"].append({"id": id, "function": {"name": name, "arguments": args}})

        if response.get("content") or response.get("tool_calls") or finished:
            yield None, finished, self.converter.conversation_message_from_dict(response)

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
            memory_model: Optional[Model] = None,
            memory_embeddings_model: Optional[Model] = None,
            tools: Optional[List[InvokableTool]] = None,
            planner: Planner = None,
            exposed_as: Optional[str] = None,
            knowledge: KnowledgeProvider = NoopKnowledge(),
            max_iterations: int = 14,
    ):
        if name is None:
            name = secrets.token_hex(12)

        validate_name(name)

        if model is None:
            model = Model(name="llama3.2")

        match node:
            case node if isinstance(node, LocalNodeProtocol):
                await Agent.start_agent_impl(
                    node,
                    instructions,
                    name,
                    model,
                    memory_model,
                    memory_embeddings_model,
                    tools,
                    planner,
                    exposed_as,
                    knowledge,
                    max_iterations,
                )
            case node if isinstance(node, RemoteNode):
                await node.start_agent(
                    instructions,
                    name,
                    model,
                    memory_model,
                    memory_embeddings_model,
                    tools,
                    planner,
                    exposed_as,
                    knowledge,
                    max_iterations,
                )
                Agent.class_logger().info(f"Successfully started agent {name} on a remote node")
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
            memory_model: Optional[Model] = None,
            memory_embeddings_model: Optional[Model] = None,
            tools: Optional[list] = None,
            planner=None,
            knowledge: Optional[KnowledgeProvider] = None,
            max_iterations: int = 14,
    ):
        if model is None:
            model = Model(name="llama3.2")

        agents = []

        match node:
            case node if isinstance(node, LocalNodeProtocol):
                for i in range(number_of_agents):
                    name = secrets.token_hex(12)
                    await Agent.start_agent_impl(
                        node,
                        instructions,
                        name,
                        model,
                        memory_model,
                        memory_embeddings_model,
                        tools,
                        copy.deepcopy(planner),
                        None,
                        copy.deepcopy(knowledge),
                        max_iterations,
                    )
                    agents.append(AgentReference(name, node))
            case node if isinstance(node, RemoteNode):
                names = await node.start_agents(
                    instructions,
                    number_of_agents,
                    model,
                    memory_model,
                    memory_embeddings_model,
                    tools,
                    planner,
                    knowledge,
                    max_iterations,
                )

                for name in names:
                    agents.append(AgentReference(name, node))

                Agent.class_logger().info("Successfully started agents on a remote node")
            case _:
                raise ValueError("Node must be either a LocalNodeProtocol or a RemoteNode")

        return agents

    @staticmethod
    async def start_agent_impl(
            node: LocalNodeProtocol,
            instructions: str,
            name: Optional[str],
            model: Model,
            memory_model: Optional[Model],
            memory_embeddings_model: Optional[Model],
            tools: Optional[List[InvokableTool]],
            planner: Planner,
            exposed_as: Optional[str],
            knowledge: Optional[KnowledgeProvider],
            max_iterations: int,
    ):
        tools_specs, tools = await prepare_tools(node, tools)
        # the memory is shared between all the agent workers
        memory = Memory()

        def agent_creator():
            return Agent(
                node,
                name,
                instructions,
                model,
                memory_model,
                memory_embeddings_model,
                tools_specs,
                tools,
                planner,
                memory,
                knowledge,
                max_iterations,
            )

        await node.start_spawner(name, agent_creator, key_extractor, None, exposed_as)

        Agent.class_logger().debug(f"Successfully started agent {name}")


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
