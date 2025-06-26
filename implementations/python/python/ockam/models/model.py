from typing import List, Optional

import litellm
from litellm import Router

from copy import deepcopy

import hashlib
import dill
import json
import os
import boto3
import threading

from ..nodes.message import ConversationMessage
from ..logging.logging import InfoContext, DebugContext

PROVIDER_ALIASES = {
    "litellm_proxy": {
        "claude-3-5-haiku-v1": "litellm_proxy/anthropic.claude-3-5-haiku-20241022-v1:0",
        "claude-3-5-sonnet-v1": "litellm_proxy/anthropic.claude-3-5-sonnet-20240620-v1:0",
        "claude-3-5-sonnet-v2": "litellm_proxy/anthropic.claude-3-5-sonnet-20241022-v2:0",
        "claude-3-7-sonnet-v1": "litellm_proxy/anthropic.claude-3-7-sonnet-20250219-v1:0",
        "claude-opus-4-v1": "litellm_proxy/anthropic.claude-opus-4-20250514-v1:0",
        "claude-sonnet-4-v1": "litellm_proxy/anthropic.claude-sonnet-4-20250514-v1:0",
        "deepseek-r1": "litellm_proxy/us.deepseek.r1-v1:0",
        "embed-english-v3": "litellm_proxy/cohere.embed-english-v3",
        "embed-multilingual-v3": "litellm_proxy/cohere.embed-multilingual-v3",
        "llama3.2": "litellm_proxy/meta.llama3-2-90b-instruct-v1:0",
        "llama3.3": "litellm_proxy/meta.llama3-3-70b-instruct-v1:0",
        "llama4-maverick": "litellm_proxy/meta.llama4-maverick-17b-instruct-v1:0",
        "llama4-scout": "litellm_proxy/meta.llama4-scout-17b-instruct-v1:0",
        "nomic-embed-text": "litellm_proxy/nomic-embed-text",
        "nova-lite-v1": "litellm_proxy/amazon.nova-lite-v1:0",
        "nova-micro-v1": "litellm_proxy/amazon.nova-micro-v1:0",
        "nova-pro-v1": "litellm_proxy/amazon.nova-pro-v1:0",
        "nova-premier-v1": "litellm_proxy/amazon.nova-premier-v1:0",
        "titan-embed-image-v1": "litellm_proxy/amazon.titan-embed-image-v1",
        "titan-embed-text-v1": "litellm_proxy/amazon.titan-embed-text-v1",
        "titan-embed-text-v2": "litellm_proxy/amazon.titan-embed-text-v2:0",
        "titan-text-express-v1": "litellm_proxy/amazon.titan-text-express-v1",
        "titan-text-lite-v1": "litellm_proxy/amazon.titan-text-lite-v1",
        "llama3.1-8b-instruct": "litellm_proxy/lambda_ai.llama3.1-8b-instruct",
    },
    "ollama": {
        "deepseek-r1": "ollama_chat/deepseek-r1",
        "llama3.2": "ollama_chat/llama3.2",
        "llama3.3": "ollama_chat/llama3.3",
        "gemma3": "ollama_chat/gemma3",
        "gemma3:27b": "ollama_chat/gemma3:27b",
        "nomic-embed-text": "ollama/nomic-embed-text",
    },
    "ollama_chat": {
        "deepseek-r1": "ollama_chat/deepseek-r1",
        "llama3.2": "ollama_chat/llama3.2",
        "llama3.3": "ollama_chat/llama3.3",
        "gemma3": "ollama_chat/gemma3",
        "gemma3:27b": "ollama_chat/gemma3:27b",
    },
    "bedrock": {
        "claude-3-5-haiku-v1": "bedrock/anthropic.claude-3-5-haiku-20241022-v1:0",
        "claude-3-5-sonnet-v1": "bedrock/anthropic.claude-3-5-sonnet-20240620-v1:0",
        "claude-3-5-sonnet-v2": "bedrock/anthropic.claude-3-5-sonnet-20241022-v2:0",
        "claude-3-7-sonnet-v1": "bedrock/anthropic.claude-3-7-sonnet-20250219-v1:0",
        "deepseek-r1": "bedrock/us.deepseek.r1-v1:0",
        "embed-english-v3": "bedrock/cohere.embed-english-v3",
        "embed-multilingual-v3": "bedrock/cohere.embed-multilingual-v3",
        "llama3.2": "bedrock/meta.llama3-2-90b-instruct-v1:0",
        "llama3.3": "bedrock/meta.llama3-3-70b-instruct-v1:0",
        "nova-lite-v1": "bedrock/amazon.nova-lite-v1:0",
        "nova-micro-v1": "bedrock/amazon.nova-micro-v1:0",
        "nova-pro-v1": "bedrock/amazon.nova-pro-v1:0",
        "titan-embed-image-v1": "bedrock/amazon.titan-embed-image-v1",
        "titan-embed-text-v1": "bedrock/amazon.titan-embed-text-v1",
        "titan-embed-text-v2": "bedrock/amazon.titan-embed-text-v2:0",
        "titan-text-express-v1": "bedrock/amazon.titan-text-express-v1",
        "titan-text-lite-v1": "bedrock/amazon.titan-text-lite-v1",
    },
}

ALL_PROVIDER_ALLOWED_FULL_NAMES = set()
for provider_aliases in PROVIDER_ALIASES.values():
    ALL_PROVIDER_ALLOWED_FULL_NAMES.update(provider_aliases.values())

BEDROCK_INFERENCE_PROFILE_MAP = {
    "amazon.nova-lite-v1:0": "us.amazon.nova-lite-v1:0",
    "amazon.nova-micro-v1:0": "us.amazon.nova-micro-v1:0",
    "amazon.nova-pro-v1:0": "us.amazon.nova-pro-v1:0",
    "amazon.nova-premier-v1:0": "us.amazon.nova-premier-v1:0",
    "anthropic.claude-3-7-sonnet-20250219-v1:0": "us.anthropic.claude-3-7-sonnet-20250219-v1:0",
    "anthropic.claude-opus-4-20250514-v1:0": "us.anthropic.claude-opus-4-20250514-v1:0",
    "anthropic.claude-sonnet-4-20250514-v1:0": "us.anthropic.claude-sonnet-4-20250514-v1:0",
    "meta.llama3-2-90b-instruct-v1:0": "us.meta.llama3-2-90b-instruct-v1:0",
    "meta.llama3-3-70b-instruct-v1:0": "us.meta.llama3-3-70b-instruct-v1:0",
    "meta.llama4-maverick-17b-instruct-v1:0": "us.meta.llama4-maverick-17b-instruct-v1:0",
    "meta.llama4-scout-17b-instruct-v1:0": "us.meta.llama4-scout-17b-instruct-v1:0",
    "us.deepseek.r1-v1:0": "us.deepseek.r1-v1:0",
}

region = None
account_id = None
cluster_id = None
init_lock = threading.Lock()

_inference_profile_cache = {}
_cache_lock = threading.Lock()

# CACHE_INFERENCE is an environment variable that allows caching of inference results, it's meant
# to be used in development and testing environments.
# During streaming inference, the results will be consumed, cached and then returned at once, so that
# any error during the execution will not cause the response to be lost.
_cache_inference = os.environ.get("CACHE_INFERENCE", "").lower() in ["1", "true", "yes", "on"]

# slightly modify the parameters to accommodate services
litellm.modify_params = True

model_list = []
for model_name in ALL_PROVIDER_ALLOWED_FULL_NAMES:
    model_list.append({"model_name": model_name, "litellm_params": {"model": model_name}})

router = Router(model_list=model_list, default_max_parallel_requests=400)


def construct_bedrock_arn(model_identifier: str, original_name: str) -> Optional[str]:
    global region, account_id, cluster_id, init_lock
    with init_lock:
        if account_id is None:
            try:
                region = os.environ.get("AWS_REGION") or os.environ.get("AWS_DEFAULT_REGION")
                if not region:
                    session = boto3.Session()
                    region = session.region_name
                sts_client = boto3.client("sts", region_name=region)
                account_id = sts_client.get_caller_identity()["Account"]

                cluster_id = os.environ.get("CLUSTER")
                if not cluster_id:
                    Model.class_logger().warning(
                        "CLUSTER is not set. Cannot automatically manage inference profiles. Returning None."
                    )
                    return None

            except Exception as e:
                Model.class_logger().warning(f"Could not construct Bedrock ARN: {e}")
                return None

    sanitized_model_name = original_name.replace(":", "_").replace(".", "_")
    cache_key = f"{cluster_id}_{sanitized_model_name}"

    with _cache_lock:
        if cache_key in _inference_profile_cache:
            return _inference_profile_cache[cache_key]
        else:
            inference_profile_name = f"{cluster_id}_{sanitized_model_name}"
            bedrock_client = boto3.client("bedrock", region_name=region)
            # Check if profile already exists
            try:
                paginator = bedrock_client.get_paginator("list_inference_profiles")
                for page in paginator.paginate(typeEquals="APPLICATION"):
                    for profile in page.get("inferenceProfileSummaries", []):
                        if profile["inferenceProfileName"] == inference_profile_name:
                            arn = profile["inferenceProfileArn"]
                            # Cache the result
                            _inference_profile_cache[cache_key] = arn
                            return arn
            except Exception as e:
                Model.class_logger().warning(f"An error occurred while listing existing inference profiles: {e}")
                return None

            # Determine the source ARN for the new profile
            if model_identifier in BEDROCK_INFERENCE_PROFILE_MAP:
                source_profile_id = BEDROCK_INFERENCE_PROFILE_MAP[model_identifier]
                model_source_arn = f"arn:aws:bedrock:{region}:{account_id}:inference-profile/{source_profile_id}"
            else:
                # This is a standard foundation model
                model_source_arn = f"arn:aws:bedrock:{region}::foundation-model/{model_identifier}"
            # Create the profile
            try:
                response = bedrock_client.create_inference_profile(
                    inferenceProfileName=inference_profile_name,
                    modelSource={"copyFrom": model_source_arn},
                    tags=[
                        {"key": "ockam.ai/clusterID", "value": cluster_id},
                        {"key": "ockam.ai/modelID", "value": model_identifier},
                    ],
                )
                created_arn = response["inferenceProfileArn"]
                # Cache the newly created ARN
                _inference_profile_cache[cache_key] = created_arn
                return created_arn
            except bedrock_client.exceptions.ConflictException:
                _inference_profile_cache[cache_key] = None
                return None
            except Exception:
                return None


class Model(InfoContext, DebugContext):
    _logger = None

    @classmethod
    def class_logger(cls):
        if cls._logger:
            return cls._logger
        else:
            from ..logging.logging import get_logger

            cls._logger = get_logger("model")
            return cls._logger

    def __init__(self, name, max_input_tokens=None, **kwargs):
        self.logger = Model.class_logger()
        if os.environ.get("LITELLM_PROXY_API_BASE"):
            provider = "litellm_proxy"
        elif os.environ.get("AWS_WEB_IDENTITY_TOKEN_FILE"):
            provider = "bedrock"
        else:
            provider = "ollama"

        self.original_name = name
        resolved_name = None
        provider_aliases = PROVIDER_ALIASES.get(provider, {})
        original_name = name

        if "/" not in name:
            if name not in provider_aliases:
                raise ValueError(f"Model alias '{original_name}' is not supported for provider '{provider}'.")

            resolved_name = provider_aliases[name]

        else:
            resolved_name = name

        if resolved_name not in ALL_PROVIDER_ALLOWED_FULL_NAMES:
            raise ValueError(
                f"Model '{original_name}' (resolved to '{resolved_name}') is not supported or enabled by any configured provider."
            )

        self.logger.debug(f"The resolved model name is '{resolved_name}'")
        self.name = resolved_name

        # TODO: test and uncomment
        # if not max_input_tokens:
        #     max_input_tokens = litellm.get_max_tokens(self.name)

        self.max_input_tokens = max_input_tokens
        self.kwargs = kwargs
        self.kwargs["drop_params"] = True

        # Extract the model identifier for both bedrock and litellm_proxy paths
        model_identifier = None
        if self.name.startswith("bedrock/"):
            model_identifier = self.name[len("bedrock/") :]
        elif self.name.startswith("litellm_proxy/"):
            model_identifier = self.name[len("litellm_proxy/") :]

        # Apply inference profile if needed
        if model_identifier and "model_id" not in kwargs:
            # Check for an explicit ARN override from the environment first
            inference_profile_arn = os.environ.get("BEDROCK_INFERENCE_PROFILE_ARN")
            if inference_profile_arn:
                self.kwargs["model_id"] = inference_profile_arn
            else:
                # If no override, use the automatic get-or-create logic
                provider, _ = resolved_name.split("/", 1)
                if provider == "bedrock" or provider == "litellm_proxy":
                    arn = construct_bedrock_arn(model_identifier, original_name)
                    if arn:
                        self.kwargs["model_id"] = arn
                    else:
                        Model.class_logger().warning(
                            f"Failed to obtain/create inference profile ARN for model_identifier: {model_identifier}. Model will be called directly."
                        )

    def count_tokens(
        self, messages: List[dict] | List[ConversationMessage], is_thinking: bool = False, tools=None
    ) -> int:
        messages, kwargs = self.prepare_llm_call(messages, is_thinking)

        # FIXME: Do kwargs we provide to acompletion could potentially affect this calculation?
        return litellm.token_counter(self.name, messages=messages, tools=tools)

    def prepare_llm_call(self, messages: List[dict] | List[ConversationMessage], is_thinking: bool, **kwargs):
        messages = normalize_messages(
            messages, is_thinking, self.support_tools(), self.support_forced_assistant_answer()
        )

        # parameters provided in kwargs will override the default parameters
        kwargs = {**self.kwargs, **kwargs}

        # sometimes an empty tools list is interpreted as "please hallucinate tools",
        if "tools" in kwargs and len(kwargs["tools"]) == 0:
            del kwargs["tools"]

        return messages, kwargs

    def support_tools(self):
        if "deepseek" in self.name:
            return False
        return True

    def support_forced_assistant_answer(self):
        return "bedrock" not in self.name and "litellm_proxy" not in self.name

    async def complete_chat(
        self,
        messages: List[dict] | List[ConversationMessage],
        stream: bool = False,
        is_thinking: bool = False,
        **kwargs,
    ):
        """
        Send a chat completion request to the model.

        :param messages: List of messages to send to the model. Each message should be a dictionary with 'role' and 'content' keys.
        :param stream: When True, the response will be streamed back as it is generated. If False, the full response will be returned at once.
        :param is_thinking: Must be true when the thinking has already started but not completed yet.
        :param kwargs: Additional parameters to pass to the model, such as temperature, max_tokens, etc.
        :rtype The response from the model, either as a full response or a stream of responses.
        """
        self.logger.info(f"Processing {len(messages)} messages with model '{self.original_name}'")
        self.logger.debug(f"Sending the following messages to the model: {messages} (stream={stream})")

        messages, kwargs = self.prepare_llm_call(messages, is_thinking, **kwargs)

        if stream:
            return self._complete_chat_stream(messages, is_thinking, **kwargs)
        else:
            return await self._complete_chat(messages, **kwargs)

    async def _complete_chat_stream(self, messages: List[dict], is_thinking: bool, **kwargs):
        chunks = None

        if _cache_inference:
            request_hash = self._hash_completion_request(self.name, messages, stream=True, kwargs=kwargs)
            file = f".recorded_inference/{request_hash}.dill"
            if os.path.exists(file):
                chunks = dill.load(open(file, "rb"))

        if chunks is None:
            if _cache_inference:
                chunks = []
                async for chunk in await self.router().acompletion(self.name, messages=messages, stream=True, **kwargs):
                    chunks.append(chunk)
                    if chunk.choices[0].finish_reason:
                        break

                os.makedirs(".recorded_inference", exist_ok=True)
                with open(file, "wb") as f:
                    f.write(dill.dumps(chunks))
            else:
                chunks = await self.router().acompletion(self.name, messages=messages, stream=True, **kwargs)

        # convert chunks to an async generator if they are a list
        if type(chunks) is list:
            chunk_list = chunks

            async def chunk_generator():
                for chunk in chunk_list:
                    yield chunk

            chunks = chunk_generator()

        async for chunk in chunks:
            if chunk.choices[0].delta.content and "<think>" in chunk.choices[0].delta.content:
                is_thinking = True
                chunk.choices[0].delta.content = chunk.choices[0].delta.content.replace("<think>", "")

            if chunk.choices[0].delta.content and "</think>" in chunk.choices[0].delta.content:
                is_thinking = False
                chunk.choices[0].delta.content = chunk.choices[0].delta.content.replace("</think>", "")

            if is_thinking:
                chunk.choices[0].delta.reasoning_content = chunk.choices[0].delta.content
                chunk.choices[0].delta.content = None
            yield chunk

    async def _complete_chat(self, messages: List[dict], **kwargs):
        response = None
        if _cache_inference:
            request_hash = self._hash_completion_request(self.name, messages, False, kwargs)
            file = f".recorded_inference/{request_hash}.dill"
            if os.path.exists(file):
                response = dill.load(open(file, "rb"))

        if response is None:
            response = await self.router().acompletion(self.name, messages=messages, stream=False, **kwargs)
            self.logger.debug(f"Got a response from model '{self.original_name}': {response}")
        self.logger.info(f"Finished processing messages with model '{self.original_name}'")

        if _cache_inference:
            os.makedirs(".recorded_inference", exist_ok=True)
            with open(file, "wb") as f:
                f.write(dill.dumps(response))

        end_think_tag = response.choices[0].message.content.find("</think>")
        if end_think_tag != -1:
            thinking_content = response.choices[0].message.content[:end_think_tag].replace("<think>", "")
            non_thinking_content = response.choices[0].message.content[end_think_tag + len("</think>") :]

            response.choices[0].message.reasoning_content = thinking_content
            response.choices[0].message.content = non_thinking_content

        return response

    async def embeddings(self, text: List[str], **kwargs) -> List[List[float]]:
        # parameters provided in kwargs will override the default parameters
        kwargs = {**self.kwargs, **kwargs}

        if _cache_inference:
            request_hash = self._hash_embedding_request(self.name, text, kwargs)
            file = f".recorded_inference/{request_hash}.dill"
            if os.path.exists(file):
                embedding = dill.load(open(file, "rb"))
                return [embedding["embedding"] for embedding in embedding.data]

        embedding = await self.router().aembedding(self.name, text, **kwargs)

        if _cache_inference:
            os.makedirs(".recorded_inference", exist_ok=True)
            request_hash = self._hash_embedding_request(self.name, text, kwargs)
            with open(f".recorded_inference/{request_hash}.dill", "wb") as f:
                f.write(dill.dumps(embedding))

        return [embedding["embedding"] for embedding in embedding.data]

    def _hash_completion_request(self, model_name: str, messages: List[dict], stream: bool, kwargs: dict) -> str:
        request = json.dumps({"model": model_name, "messages": messages, "stream": stream, **kwargs}, sort_keys=True)
        return hashlib.sha256(request.encode("utf-8")).hexdigest()

    def _hash_embedding_request(self, model_name: str, text: List[str], kwargs: dict) -> str:
        request = json.dumps({"model": model_name, "text": text, **kwargs}, sort_keys=True)
        return hashlib.sha256(request.encode("utf-8")).hexdigest()

    def router(self):
        global router

        return router


def normalize_messages(
    messages: List[dict] | List[ConversationMessage],
    is_thinking: bool,
    tools_supported: bool,
    forced_assistant_answer_supported: bool,
) -> List[dict]:
    messages = deepcopy(messages)

    # convert if the messages are typed as ConversationMessage
    if len(messages) > 0:
        if not isinstance(messages[0], dict):
            messages = [
                {
                    "role": message.role.value,
                    "content": message.content,
                }
                for message in messages
            ]

    # change type to list[dict]
    messages: List[dict]

    # remove any empty text content
    for message in messages:
        if "content" in message and len(message["content"]) == 0:
            del message["content"]

    # remove extra fields: phase and thinking
    for message in messages:
        if "phase" in message:
            del message["phase"]
        if "thinking" in message:
            del message["thinking"]

    if tools_supported:
        for message in messages:
            # remove any tool calls when empty; this avoids litellm workarounds when they are not needed
            if "tool_calls" in message:
                if len(message["tool_calls"]) == 0:
                    del message["tool_calls"]
    else:
        for message in messages:
            # convert any tool role to assistant
            if message["role"] == "tool":
                message["role"] = "assistant"

            # remove any tool calls
            if "tool_calls" in message:
                del message["tool_calls"]

    # remove scope and conversation from messages before sending them over litellm
    for message in messages:
        if "scope" in message:
            del message["scope"]
        if "conversation" in message:
            del message["conversation"]

    # compact messages with the same fields except 'content'
    if len(messages) > 0:
        compacted = [messages[0]]
        for msg in messages[1:]:
            last = compacted[-1]
            # Compare all keys except 'content' and merge if equal
            keys_to_compare = set(msg.keys()) | set(last.keys())
            keys_to_compare.discard("content")
            if all(msg.get(k) == last.get(k) for k in keys_to_compare):
                # Merge content if both have it
                if "content" in last and "content" in msg:
                    last["content"] += msg["content"]
                elif "content" in msg:
                    last["content"] = msg["content"]
                else:
                    # neither has content
                    pass
            else:
                compacted.append(msg)
        messages = compacted

    # delete messages without useful information
    new_messages = []
    for message in messages:
        match message.get("role"):
            case "system" | "user" if not message.get("content", None):
                continue
            case "assistant" if not message.get("content", None) and not message.get("tool_calls", None):
                continue
            case "tool" if not message.get("name", None):
                continue
            case _:
                new_messages.append(message)
    messages = new_messages

    # if the last message is an assistant message, we need to change it to user when
    # force_assistant_answer is not supported
    if not forced_assistant_answer_supported and len(messages) > 0 and messages[-1]["role"] == "assistant":
        messages[-1]["role"] = "user"

    # if thinking, we need to add <think> tag at the beginning of the last message
    if is_thinking and len(messages) > 0:
        if "content" in messages[-1]:
            messages[-1]["content"] = "<think>" + messages[-1]["content"]
        else:
            messages[-1]["content"] = "<think>"

    return messages
