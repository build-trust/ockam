#!/bin/bash
set -ex

# This script is used as an entrypoint to a docker container built using ../ockam.dockerfile.

# Run `ockam project enroll ...`
#
# The `project enroll` command creates a new vault and generates a cryptographic identity with
# private keys stored in that vault.
#
# The enrollment ticket includes routes and identifiers for the project membership authority
# and the project’s node that offers the relay service.
#
# The enrollment ticket also includes an enrollment token. The project enroll command
# creates a secure channel with the project membership authority and presents this enrollment token.
# The authority enrolls presented identity and returns a project membership credential.
#
# The command, stores this credential for later use and exits.
ockam project enroll "$ENROLLMENT_TICKET"

# Create an ockam node.
#
# Create an access control policy that only allows project members that possesses a credential with
# attribute mongodb-outlet="true" to connect to TCP Portal Inlets on this node.
#
# Create a TCP Portal Inlet to MongoDB.
# This makes the remote MongoDB available on all localhost IPs at - 0.0.0.0:17017
ockam node create
ockam policy create --resource-type tcp-inlet --expression '(= subject.mongodb-outlet "true")'
ockam tcp-inlet create --from 0.0.0.0:17017 --via mongodb

# Run the container forever.
tail -f /dev/null
