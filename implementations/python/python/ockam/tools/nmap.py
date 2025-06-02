import asyncio
import cattr
import json

from dataclasses import dataclass
from enum import Enum
from typing import Union

from ..nodes import NodeProtocol
from ..ockam_in_rust_for_python import debug


class NmapWorker:
    ADDRESS = "nmap"

    def __init__(self):
        self.converter = create_worker_converter()

    @staticmethod
    async def start(node: NodeProtocol):
        await node.start_worker(NmapWorker.ADDRESS, NmapWorker())

    # TODO: Make it a spawner so we can parallelize execution
    async def handle_message(self, context, message):
        try:
            request = json.loads(message)

            request = self.converter.structure(request, NmapRequest)

            output = None
            if isinstance(request, NmapRequestGetManual):
                output = await get_nmap_manual()

            if isinstance(request, NmapRequestRunCommand):
                output = await run_nmap(request.body)

            if not output:
                raise ValueError(f"Unknown request type: {request.request_type}")

            response = NmapResponseSuccess(response_type=NmapResponseType.SUCCESS.value, body=output)
            response = self.converter.unstructure(response)
            response = json.dumps(response)
            await context.reply(response)

        except Exception as e:
            response = NmapResponseError(response_type=NmapResponseType.ERROR.value, body=str(e))
            response = self.converter.unstructure(response)
            response = json.dumps(response)
            await context.reply(response)


async def get_nmap_manual():
    process = await asyncio.create_subprocess_exec(
        "man",
        "nmap",
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )

    stdout, stderr = await process.communicate()

    if stderr:
        error = stderr.decode()
        raise Exception(f"Nmap command failed: {error}")

    return stdout.decode()


async def run_nmap(command: str) -> str:
    """
    Execute nmap command and stdout.

    :param command: nmap arguments
    :return: standard output of nmap command
    """
    debug(f"Running nmap command: {command}")

    arguments = command.split(" ")

    if not arguments:
        raise Exception("No arguments provided")

    if arguments[0] == "nmap":
        arguments.pop(0)

    process = await asyncio.create_subprocess_exec(
        "nmap",
        *arguments,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )

    stdout, stderr = await process.communicate()

    if stderr:
        error = stderr.decode()
        raise Exception(f"Nmap command failed: {error}")

    return stdout.decode()


class NmapClient:
    def __init__(self, node):
        self.node = node
        self.converter = create_client_converter()

    async def get_manual(self, timeout=None):
        request = NmapRequestGetManual(request_type=NmapRequestType.GET_MANUAL.value)
        request = self.converter.unstructure(request)
        request = json.dumps(request)
        response = await self.node.send_and_receive(NmapWorker.ADDRESS, request, timeout=timeout)
        return self.parse_response(response)

    async def run_command(self, command, timeout=None):
        request = NmapRequestRunCommand(body=command, request_type=NmapRequestType.RUN_COMMAND.value)
        request = self.converter.unstructure(request)
        request = json.dumps(request)
        response = await self.node.send_and_receive(NmapWorker.ADDRESS, request, timeout=timeout)
        return self.parse_response(response)

    def parse_response(self, response):
        response = json.loads(response)
        response = self.converter.structure(response, NmapResponse)

        if isinstance(response, NmapResponseSuccess):
            return response.body

        if isinstance(response, NmapResponseError):
            raise Exception(response.body)

        raise ValueError("Unknown response type")


class NmapRequestType(Enum):
    GET_MANUAL = "get_manual"
    RUN_COMMAND = "run_command"


@dataclass
class NmapRequestGetManual:
    request_type: str


@dataclass
class NmapRequestRunCommand:
    request_type: str
    body: str


NmapRequest = Union[NmapRequestGetManual, NmapRequestRunCommand]


class NmapResponseType(Enum):
    SUCCESS = "success"
    ERROR = "error"


@dataclass
class NmapResponseSuccess:
    response_type: str
    body: str


@dataclass
class NmapResponseError:
    response_type: str
    body: str


NmapResponse = Union[NmapResponseSuccess, NmapResponseError]


def create_worker_converter():
    converter = cattr.Converter()

    def structure_request(obj, _):
        obj_type = obj.get("request_type")
        if obj_type == NmapRequestType.GET_MANUAL.value:
            return converter.structure(obj, NmapRequestGetManual)
        if obj_type == NmapRequestType.RUN_COMMAND.value:
            return converter.structure(obj, NmapRequestRunCommand)
        else:
            raise ValueError(f"Unknown request type: {obj_type}")

    converter.register_structure_hook(NmapRequest, structure_request)

    return converter


def create_client_converter():
    converter = cattr.Converter()

    def structure_response(obj, _):
        obj_type = obj.get("response_type")
        if obj_type == NmapResponseType.SUCCESS.value:
            return converter.structure(obj, NmapResponseSuccess)
        if obj_type == NmapResponseType.ERROR.value:
            return converter.structure(obj, NmapResponseError)
        else:
            raise ValueError(f"Unknown response type: {obj_type}")

    converter.register_structure_hook(NmapResponse, structure_response)

    return converter
