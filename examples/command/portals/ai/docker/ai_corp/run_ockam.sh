#!/bin/bash
set -ex

# This script is used as an entrypoint to a docker container built using ../ockam.dockerfile.

# Run `ockam project enroll ...`
#
# The `project enroll` command creates a new vault and generates a cryptographic identity with
# private keys stored in that vault.
#
# The enrollment ticket includes routes and identitifiers for the project membership authority
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
# Create an encrypted relay to this node in the project at address: ai.
# The relay makes this node reachable by other project members.
#
# Create an access control policy that only allows project members that possesses a credential with
# attribute ai-inlet="true" to connect to TCP Portal Outlets on this node.
#
# Create a TCP Portal Outlet to ai API at - ai:3000.
ockam node create
ockam relay create ai
ockam policy create --resource tcp-outlet --expression '(= subject.ai-inlet "true")'
ockam tcp-outlet create --to ai:3000

# Run the container forever.
tail -f /dev/null
