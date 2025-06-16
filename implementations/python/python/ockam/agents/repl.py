import asyncio
import secrets
import re
import os

from ockam.nodes.message import StreamedConversationSnippet
from ockam.agents.socket_address import parse_host_and_port


DEFAULT_HOST = os.environ.get("DEFAULT_HOST_REPL", "127.0.0.1")
DEFAULT_PORT = int(os.environ.get("DEFAULT_PORT_REPL", "7000"))


class Repl:
    _logger = None

    @classmethod
    def logger(cls):
        if cls._logger:
            return cls._logger
        else:
            from ..logging.logging import get_logger

            cls._logger = get_logger("repl")
            return cls._logger

    def __init__(
        self, agent_reference, listen_address=f"{DEFAULT_HOST}:{DEFAULT_PORT}", functions=None, timeout=120, stream=True
    ):
        self.logger = Repl.logger()
        self.functions = functions or {}
        self.host = DEFAULT_HOST
        self.port = DEFAULT_PORT
        self.set_host_and_port(listen_address)
        self.agent_reference = agent_reference
        self.server = None
        self.scope = None
        self.conversation = None
        self.timeout = timeout
        self.stream = stream

        self.short_help_text = "Type ':help' or ':h' to see this message again.\nType ':quit' or ':q' to disconnect."

        self.help_text = (
            "Type ':help' or ':h' to see this message again.\n"
            "Type ':scope' or ':s' to set the scope for future interactions.\n"
            "Type ':conversation' or ':c' to set the conversation for future interactions.\n"
            "Type ':functions' to see the list of available functions.\n"
            "Type ':run' to run a predefined function.\n"
            "Type ':quit' or ':q' to disconnect."
        )

    # TODO: should be on Reference class
    @staticmethod
    async def start(
        agent_reference, listen_address=f"{DEFAULT_HOST}:{DEFAULT_PORT}", functions=None, timeout=None, stream=True
    ):
        Repl.logger().info("starting the REPL")
        repl = Repl(agent_reference, listen_address, functions, timeout, stream)
        # start the repl in a separate thread since we might also have a HTTP server running
        asyncio.create_task(repl.start_impl())

    def set_host_and_port(self, listen_address):
        try:
            host, port = parse_host_and_port(listen_address)
            self.host = host
            self.port = port
        except ValueError:
            raise ValueError(f"Invalid listen_address: {listen_address}")

    async def list_functions(self, writer):
        self.logger.debug("list functions")
        functions_list = "\n".join([f"{name}" for name in self.functions.keys()])
        await self.write_repl_message(writer, functions_list)

    async def update_scope(self, message, writer):
        try:
            _, scope = message.split(" ", 1)
            self.scope = scope
            await self.write_repl_message(writer, f"Scope set to {self.scope}")
        except ValueError:
            await self.write_repl_message(writer, "Invalid scope format. Use ':scope <scope>'.")

    async def update_conversation(self, message, writer):
        try:
            _, conversation = message.split(" ", 1)
            self.conversation = conversation
            await self.write_repl_message(writer, f"Conversation set to {self.conversation}")
        except ValueError:
            await self.write_repl_message(writer, "Invalid conversation format. Use ':conversation <conversation>'.")

    async def write_repl_message(self, writer, message):
        """
        Write a REPL instruction with newlines at the end to leave some space for the prompt
        """
        message = (message + "\n\n").encode()
        writer.write(f"{len(message)}\n".encode())
        writer.write(message)
        await writer.drain()

    async def write(self, writer, message):
        """
        Write a message to the output with a length encoding
        """

        for word in re.findall(r"\S+|\s+", message):
            word = word.encode()
            writer.write(f"{len(word)}\n".encode())
            writer.write(word)
            await writer.drain()
            await asyncio.sleep(0.03)

    async def handle_client(self, reader, writer):
        self.scope = secrets.token_hex(16)
        self.conversation = secrets.token_hex(12)

        _client_address = writer.get_extra_info("peername")
        welcome_message = (
            "\nWelcome to Ockam 👋\n\n"
            f"You are connected to your agent - {self.agent_reference.node_name}/{self.agent_reference.name}\n"
            "Ask it a question or give it a task.\n\n"
        )

        await self.write_repl_message(writer, welcome_message + self.short_help_text)

        try:
            while True:
                data = await reader.readline()
                if not data:
                    break

                # Multiline support
                separator = b'"""\n'
                if data == separator:
                    data = await reader.readuntil(separator)
                    data = data[: -len(separator)]

                message = data.decode().strip()
                if message.lower() in (":quit", ":q"):
                    break

                elif message.lower() in (":help", ":h"):
                    await self.write_repl_message(writer, self.help_text)
                    continue

                elif message.lower() == ":functions":
                    await self.list_functions(writer)
                    continue

                elif message.lower().split()[0] in (":scope", ":s"):
                    await self.update_scope(message, writer)
                    continue

                elif message.lower().split()[0] in (":conversation", ":c"):
                    await self.update_conversation(message, writer)
                    continue

                elif message.lower().split()[0] == ":run":
                    parts = message.split(" ")
                    if len(parts) < 2 or len(parts) > 2:
                        await self.write_repl_message(writer, "Usage: :run <function_name>")
                    elif parts[1] in self.functions:
                        function_name = parts[1]
                        reply = await self.functions[function_name]()
                        await self.write_repl_message(writer, reply)
                    else:
                        await self.write_repl_message(writer, "Function not found.")
                    continue

                if self.stream:
                    buffer = ""
                    async for response in self.agent_reference.send_stream(
                        message, scope=self.scope, conversation=self.conversation, timeout=self.timeout
                    ):
                        # even if we make a streaming request, the response might not be streaming if the downstream
                        # agent does not support streaming
                        if type(response) is StreamedConversationSnippet:
                            if len(response.snippet.messages) > 0:
                                received = response.snippet.messages[0].content
                                buffer += received
                                if len(buffer) > 20:
                                    await self.write(writer, buffer)
                                    buffer = ""
                            if response.finished:
                                await self.write(writer, buffer)
                                # indicate the end of the stream
                                writer.write(b"0\n")
                                await writer.drain()
                                break
                        else:
                            received = response.messages[0].content
                            await self.write(writer, received)
                            # indicate the end of the stream
                            writer.write(b"0\n")
                            await writer.drain()
                            break
                else:
                    reply = await self.agent_reference.send(
                        message, scope=self.scope, conversation=self.conversation, timeout=self.timeout
                    )
                    await self.write(writer, reply[0].content)
                    # indicate that there are no more messages
                    writer.write(b"0\n")
                    await writer.drain()

        except Exception as e:
            self.logger.error(f"Error: {e}")
            print(f"Error: {e}", flush=True)
        finally:
            writer.close()
            await writer.wait_closed()

    async def start_impl(self):
        self.server = await asyncio.start_server(self.handle_client, self.host, self.port)
        async with self.server:
            await self.server.serve_forever()

    async def stop(self):
        self.logger.info("stop the REPL")
        if self.server:
            self.server.close()
            await self.server.wait_closed()
