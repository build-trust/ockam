import json
import traceback
from copy import deepcopy
from typing import Optional, AsyncGenerator, Tuple, Dict

import secrets
import copy

from .agent_memory_knowledge import AgentMemoryKnowledge
from ..knowledge.noop import NoopKnowledge
from ..nodes import RemoteNode, LocalNodeProtocol
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
)
from .names import validate_name

from ..ockam_in_rust_for_python import info, warn, debug

from ..logging.logging import get_logging_config, warning, error
import logging.config

logging.config.dictConfig(get_logging_config())
logger = logging.getLogger("agent")


class Agent:
    # for type hints
    memory_knowledge: Optional[AgentMemoryKnowledge]

    def __init__(
        self,
        node: LocalNodeProtocol,
        name: str,
        instructions: str,
        model: Model,
        memory_model: Optional[Model],
        memory_embeddings_model: Optional[Model],
        tool_specs,
        tools: list,
        planner: Planner,
        memory: Memory,
        knowledge: KnowledgeProvider,
        maximum_iterations: int,
    ):
        logger.info("starting agent")
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
            logger.info("received a message")
            logger.debug(f"the message is {message}")

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
                    logger.error(f"unexpected message: {message}")
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
        from ..agents import ConversationResponse

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

        # If there is a planner, initialize a plan
        plan = None
        if self.planner is not None:
            plan = await self.planner.plan(messages, contextual_knowledge, stream)
        else:
            for message in messages:
                await self.remember(scope, conversation, message)

        replied = False
        whole_response_snippet = ConversationSnippet(scope, conversation, [])
        iteration = 0
        while True:
            if plan is None:
                if replied:
                    break
            else:
                next_steps = plan.next_step(await self.get_messages_only(conversation, scope), contextual_knowledge)

                replied = False
                plan_completed = True
                async for next_step in next_steps:
                    if next_step is None:
                        break
                    plan_completed = False
                    contextual_knowledge = await self.knowledge.search_knowledge(scope, conversation, next_step.content)
                    if next_step.phase == Phase.PLANNING:
                        await self.remember(scope, conversation, next_step)
                    if stream:
                        yield response.make_snippet(next_step)
                    else:
                        whole_response_snippet.messages.append(next_step)
                if plan_completed:
                    # we executed the whole plan
                    break

            tool_calls = []
            # Call the model
            model_response: AssistantMessage
            async for error, finished, model_response in self.complete_chat(
                scope, conversation, contextual_knowledge, stream=stream
            ):
                # Break the loop, if the model complete_chat call returned an error.
                if error:
                    yield response.make_snippet(error)
                    return

                await self.remember(scope, conversation, model_response)

                if len(model_response.tool_calls) > 0:
                    if stream:
                        yield response.make_snippet(model_response)
                    else:
                        whole_response_snippet.messages.append(model_response)

                    # postpone tool calls until the end of the response
                    tool_calls.extend(model_response.tool_calls)
                else:
                    if stream:
                        yield response.make_snippet(model_response)
                        if finished:
                            replied = True
                            break
                    else:
                        whole_response_snippet.messages.append(model_response)
                        replied = True
                        break

            for tool_call in tool_calls:
                error, tool_call_response = await self.call_tool(tool_call, scope, conversation)
                if error:
                    warn(f"MCP call failed: {error}")
                # remember the tool being called
                await self.remember(scope, conversation, tool_call_response)

                # we need another iteration to process the tool call response
                replied = False

            # Move to the next iteration
            iteration += 1

            # Break the loop, if the loop has reached maximum_iterations.
            # Send an error as a reply.
            if iteration == self.maximum_iterations:
                yield Error(str(RuntimeError("Reached maximum_iterations")))

        if stream:
            yield response.make_finished_snippet()
        else:
            yield whole_response_snippet

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
            if skipped_message_history:
                await self.memory_knowledge.add(
                    scope=scope, conversation=conversation, messages=skipped_message_history
                )

            # We should actually clear knowledge in this function each time...
            memory_knowledge = await self.memory_knowledge.search(scope, conversation, query)
            warning(f"RELATED MEMORY for query: {query}:\n{memory_knowledge}.")
            if memory_knowledge:
                memory_knowledge = [
                    {
                        "role": "system",
                        "content": f"The following information from your past interactions with the user could be useful to answer properly:\n{memory_knowledge}",
                    }
                ]
            else:
                memory_knowledge = []

            error(f"PROMPTS: {prompts}")
            error(f"MEMORY_KNOWLEDGE: {memory_knowledge}")
            error(f"VISIBLE_MEMORY_HISTORY: {visible_message_history}")
            input_context = prompts + memory_knowledge + visible_message_history
            number_of_tokens = self.model.count_tokens(
                tools=self.tool_specs,
                messages=input_context,
            )

            warning(
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

        response = await self.model.complete_chat(tools=self.tool_specs, messages=input_context, stream=stream)
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
                debug(f"finished streaming reply with reason {choice.get('finish_reason', 'unknown')}")
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

    async def call_tool(self, tool_call: ToolCall, scope: str, conversation: str):
        tools = self.tools

        id = tool_call.id
        name = tool_call.function.name
        args = tool_call.function.arguments

        if name not in tools:
            error_response = f"Tool '{name}' not found."
            error = Error(error_response)
            return error, tool_call_response(id, name, error_response)

        tool = tools[name]
        response = await tool.invoke(args)
        return None, tool_call_response(id, name, response)

    @staticmethod
    async def start(
        node: NodeProtocol,
        instructions: str,
        name: Optional[str] = None,
        model: Model = Model(name="llama3.2"),
        memory_model: Optional[Model] = None,
        memory_embeddings_model: Optional[Model] = None,
        tools: Optional[list] = None,
        planner: Planner = None,
        exposed_as: Optional[str] = None,
        knowledge: KnowledgeProvider = NoopKnowledge(),
        max_iterations: int = 14,
    ):
        if name is None:
            name = secrets.token_hex(12)

        validate_name(name)

        if isinstance(node, LocalNodeProtocol):
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
        elif isinstance(node, RemoteNode):
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

            info(f"Successfully started agent {name} on a remote node")
        else:
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
        model: Model = Model(name="llama3.2"),
        memory_model: Optional[Model] = None,
        memory_embeddings_model: Optional[Model] = None,
        tools: Optional[list] = None,
        planner=None,
        knowledge: Optional[KnowledgeProvider] = None,
        max_iterations: int = 14,
    ):
        agents = []

        if isinstance(node, LocalNodeProtocol):
            for i in range(number_of_agents):
                name = secrets.token_hex(12)
                await Agent.start_agent_impl(
                    node,
                    instructions,
                    name,
                    copy.deepcopy(model),
                    copy.deepcopy(memory_model),
                    copy.deepcopy(memory_embeddings_model),
                    copy.deepcopy(tools),
                    copy.deepcopy(planner),
                    None,
                    copy.deepcopy(knowledge),
                    max_iterations,
                )
                agents.append(AgentReference(name, node))
        elif isinstance(node, RemoteNode):
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

            info("Successfully started agents on a remote node")
        else:
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
        tools: Optional[list],
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

        info(f"Starting agent {name}")

        await node.start_spawner(name, agent_creator, key_extractor, None, exposed_as)

        info(f"Successfully started agent {name}")


def key_extractor(message):
    message = json.loads(message)
    scope = message.get("scope")
    conversation = message.get("conversation")
    if conversation:
        if scope:
            return f"{scope}/{conversation}"
    return None


async def prepare_tools(node, tools):
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


def tool_call_response(id: str, name: str, response: str):
    return {"role": "tool", "tool_call_id": id, "name": name, "content": response}
