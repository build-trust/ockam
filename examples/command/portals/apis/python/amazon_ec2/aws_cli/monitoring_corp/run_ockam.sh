#!/bin/bash
set -ex

# Change into ec2-user's home directory and use sudo to run the commands as ec2-user
cd /home/ec2-user
sudo -u ec2-user bash << 'EOS'
set -ex

# Install Ockam Command
export OCKAM_VERSION="$OCKAM_VERSION"
curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
source "$HOME/.ockam/env"

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
# The authority enrolls the presented identity and returns a project membership credential.
#
# The command stores this credential for later use and exits.
ockam project enroll "$ENROLLMENT_TICKET"

# Create an ockam node.
#
# Create an encrypted relay to this node in the project at - localhost:5000.
# The relay makes this node reachable by other project members.
#
# Create an access control policy that only allows project members that possesses a credential with
# attribute monitoring-api-inlet="true" to connect to TCP Portal Outlets on this node.
#
# Create a TCP Portal Outlet to the python flask api at - localhost:5000.
ockam node create
ockam relay create monitoring-api
ockam policy create --resource-type tcp-outlet --expression '(= subject.monitoring-api-inlet "true")'
ockam tcp-outlet create --to localhost:5000

EOS
