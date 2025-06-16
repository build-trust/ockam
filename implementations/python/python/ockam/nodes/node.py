import asyncio
import os
import secrets
import litellm
from typing import Awaitable, Callable, Optional

from .local import LocalNode
from .manager import RemoteManager

from ..ockam_in_rust_for_python import Node as RustNode


class Node:
    _logger = None

    @classmethod
    def logger(cls):
        if cls._logger:
            return cls._logger
        else:
            from ..logging.logging import get_logger

            cls._logger = get_logger("node")
            return cls._logger

    @staticmethod
    def start(
        main: Optional[Callable[[LocalNode], Awaitable[None]]] = None,
        name: Optional[str] = None,
        ticket: Optional[str] = None,
        allow: Optional[str] = None,
        http_server=None,
        wait_until_interrupted: bool = True,
        cache_secure_channels: bool = False,
        use_local_db: bool = False,
        llm_debug: bool = False,
        **kwargs,
    ):
        # This will make the node use a local SQLite database instead of the Postgres database
        if use_local_db:
            os.environ.pop("OCKAM_DATABASE_INSTANCE", None)
            os.environ.pop("OCKAM_DATABASE_INSTANCE", None)
            os.environ.pop("OCKAM_DATABASE_INSTANCE", None)
            os.environ["OCKAM_TELEMETRY_EXPORT"] = "false"
            os.environ["OCKAM_SQLITE_IN_MEMORY"] = "true"

        if llm_debug:
            litellm._turn_on_debug()

        name = pick_name(name)
        ticket = pick_ticket(ticket)
        cluster = pick_cluster()

        if allow is None and cluster:
            allow = f"message.is_local or cluster={cluster}"

        if wait_until_interrupted or main is None:
            main = wait_until_interrupted_decorator(main)

        async def start_node(node):
            node = LocalNode(node)
            await RemoteManager(node).start()

            if http_server is None:
                from .. import agents

                asyncio.create_task(agents.HttpServer().start(node))
            else:
                asyncio.create_task(http_server.start(node))

            await main(node)

        try:
            Node.logger().info(f"start node '{name}'")
            RustNode.start(
                start_node, name=name, ticket=ticket, allow=allow, cache_secure_channels=cache_secure_channels, **kwargs
            )
        except KeyboardInterrupt:
            Node.logger().debug("shutting down node due to KeyboardInterrupt")
            Node.logger().debug(f"\nNode {name} shutting down")


def wait_until_interrupted_decorator(func=None):
    async def wrapper(*args, **kwargs):
        node = args[0]
        if func is not None:
            await func(*args, **kwargs)
        await node.interrupted()

    return wrapper


def pick_cluster():
    return os.getenv("CLUSTER")


def pick_name(name):
    cluster = pick_cluster()
    zone = os.getenv("ZONE")
    node = os.getenv("NODE")

    if name is not None:
        return name
    elif cluster is not None and zone is not None and node is not None:
        return f"{cluster}-{zone}-{node}"
    else:
        return secrets.token_hex(12)


def pick_ticket(ticket):
    env_ticket = os.getenv("ENROLLMENT_TICKET")

    if ticket is not None:
        return ticket
    elif env_ticket is not None:
        return env_ticket
    else:
        return None
