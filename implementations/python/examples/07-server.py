from ockam import Node, McpServer, HttpServer

"""
    Second part of example 07. This simply starts a McpServer worker and makes it accessible via tcp.
    The RemoteManager is responsible for creating agents on that node when a StartAgent request is received.
    Once an agent is started, the McpServer will direct messages to it if that agent is exposed as a tool.
"""

Node.start(
    mcp_server=McpServer(listen_address="127.0.0.1:8000"),
    http_server=HttpServer(listen_address="localhost:9000"),
)

# OCKAM_SQLITE_IN_MEMORY=1 CLUSTER=acme NODE=node2 ENROLLMENT_TICKET="$(ockam project ticket --relay node2 --attribute cluster=acme)" uv run examples/07-server.py
