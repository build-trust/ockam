import json
import traceback
from typing import Optional, AsyncGenerator

import regex as re
import secrets

from ..nodes import RemoteNode
from ..planning import Planner

from ..knowledge import SearchResults, KnowledgeProvider
from ..memory import Memory
from ..models import Model
from ..nodes import NodeInterface
from ..nodes.node import Node
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
)

from ..ockam_in_rust_for_python import info, warn, debug


class Agent:
    def __init__(
        self,
        node: Node,
        name: str,
        instructions: str,
        model: Model,
        tool_specs,
        tools: list,
        planner: Planner,
        memory: Memory,
        knowledge: KnowledgeProvider,
        max_knowledge_size: int,
    ):
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
                    reply = Error(f"Unexpected Message: {message}")

                if reply is not None:
                    context.reply(self.converter.message_to_json(reply))
        except Exception as e:
            traceback.print_exc()
            error = Error(str(e))
            context.reply(self.converter.message_to_json(error))

    async def handle__get_identifier_request(self, message: GetIdentifierRequest) -> GetIdentifierResponse:
        name_snake_case = self.name.lower().replace(" ", "_")
        node_identifier = self.node.identifier()
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

        messages = snippet.messages

        query = ""
        for message in messages:
            if message.role == ConversationRole.USER:
                query = message.content
            await self.remember(scope, conversation, message)

        contextual_knowledge = None
        if self.knowledge is not None:
            contextual_knowledge = await self.add_knowledge_search(query)

        # If there is a planner, initialize a plan
        plan = None
        if self.planner is not None:
            plan = await self.planner.plan(await self.get_messages_only(conversation, scope), contextual_knowledge)

        reply = None
        iteration = 0
        while True:
            if plan is None:
                if reply is not None:
                    break
            else:
                next_steps = await plan.next_step(
                    await self.get_messages_only(conversation, scope), contextual_knowledge
                )
                if reply and next_steps is None:
                    break
                if next_steps is not None:
                    for next_step in next_steps:
                        contextual_knowledge = await self.add_knowledge_search(next_step.content)
                        await self.remember(scope, conversation, next_step)

            # Call the model
            model_response: AssistantMessage
            async for error, part_nb, finished, model_response in self.complete_chat(
                scope, conversation, contextual_knowledge, stream=stream
            ):
                # Break the loop, if the model complete_chat call returned an error.
                if error:
                    response_snippet = ConversationSnippet(scope, conversation, [error])
                    if stream:
                        yield StreamedConversationSnippet(response_snippet, part_nb, finished=True)
                    else:
                        yield response_snippet
                    break

                await self.remember(scope, conversation, model_response)

                if len(model_response.tool_calls) > 0:
                    for tool_call in model_response.tool_calls:
                        error, tool_call_response = await self.call_tool(tool_call, scope, conversation)
                        if error:
                            warn(f"MCP call failed: {error}")
                        # remember the tool being called
                        await self.remember(scope, conversation, tool_call_response)
                else:
                    reply = model_response
                    response_snippet = ConversationSnippet(scope, conversation, [reply])
                    if stream:
                        yield StreamedConversationSnippet(response_snippet, part_nb, finished=finished)
                    else:
                        yield response_snippet

            # Move to the next iteration
            iteration += 1

            # Break the loop, if the loop has reached maximum_iterations.
            # Send an error as a reply.
            if iteration == self.maximum_iterations:
                yield Error(str(RuntimeError("Reached maximum_iterations")))

    async def get_messages_only(self, conversation, scope) -> list[ConversationMessage]:
        messages: list[dict] = self.memory.get_messages_only(scope, conversation)
        return [self.converter.conversation_message_from_dict(m) for m in messages]

    async def add_knowledge_search(self, query: str) -> Optional[str]:
        """
        Search the knowledge base for relevant information and adds it to the search results.
        """
        if self.knowledge is None:
            return None

        if not query or len(query) == 0:
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
    ) -> AsyncGenerator[AssistantMessage, None]:
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
            n = 0
            async for chunk in response:
                n += 1
                yield await self.send_response(chunk, n)
        else:
            yield await self.send_response(response)

    async def send_response(self, response, part_nb=0) -> (Error | None, int, bool, AssistantMessage):
        """
        This function converts the model response into an AssistantMessage.
        When streaming is used the function also returns:
            - the current part number
            - a boolean indicating if this is the last part of the response
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

        return None, part_nb, finished, self.converter.conversation_message_from_dict(response)

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
        node: NodeInterface,
        instructions: str,
        name: Optional[str] = None,
        model: Model = Model(name="llama3.2"),
        tools: Optional[list] = None,
        planner: Planner = None,
        exposed_as: Optional[str] = None,
        knowledge: Optional[KnowledgeProvider] = None,
        max_knowledge_size: int = 4096,
    ):
        if name is None:
            name = secrets.token_hex(12)

        if exposed_as is not None and re.match(r"^[a-z0-9_-]+$", name) is None:
            raise ValueError("Agent name may only contain [a-z0-9_-] when exposed")

        if node.is_remote:
            await node.start_agent(instructions, name, model, tools, planner, exposed_as, knowledge, max_knowledge_size)

            info(f"Successfully started agent {name} on a remote node")
        else:
            await Agent.start_agent_impl(
                node, instructions, name, model, tools, planner, exposed_as, knowledge, max_knowledge_size
            )

        return AgentReference(name, node)

    @staticmethod
    async def stop(node, name):
        # RemoteNode stop_worker is async, while the local node stop_worker is not
        if node.is_remote:
            await node.stop_worker(name)
        else:
            node.stop_worker(name)

    @staticmethod
    async def start_many(
        node: NodeInterface,
        instructions: str,
        number_of_agents: int,
        model: Model = Model(name="llama3.2"),
        tools: Optional[list] = None,
        planner=None,
        knowledge: Optional[KnowledgeProvider] = None,
        max_knowledge_size: int = 4096,
    ):
        agents = []

        if isinstance(node, RemoteNode):
            names = await node.start_agents(
                instructions, number_of_agents, model, tools, planner, knowledge, max_knowledge_size
            )

            for name in names:
                agents.append(AgentReference(name, node))

            info("Successfully started agents on a remote node")
        else:
            for i in range(number_of_agents):
                name = secrets.token_hex(12)
                await Agent.start_agent_impl(
                    node, instructions, name, model, tools, planner, None, knowledge, max_knowledge_size
                )
                agents.append(AgentReference(name, node))

        return agents

    @staticmethod
    async def start_agent_impl(
        node: NodeInterface,
        instructions: str,
        name: Optional[str],
        model: Model,
        tools: Optional[list],
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
