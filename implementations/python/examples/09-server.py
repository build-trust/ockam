from ockam import Node

Node.start()

# OCKAM_SQLITE_IN_MEMORY=1 CLUSTER=acme NODE=node2 ENROLLMENT_TICKET="$(ockam project ticket --relay node2 --attribute cluster=acme)" uv run examples/09-server.py
