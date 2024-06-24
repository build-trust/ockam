#!/bin/bash
set -ex

# Change into ec2-user's home directory and use sudo to run the commands as ec2-user
cd /home/ec2-user
sudo -u ec2-user bash << 'EOS'
set -ex

# Install MongoDB https://www.mongodb.com/docs/manual/tutorial/install-mongodb-on-amazon/#install-mongodb-community-edition
sudo tee /etc/yum.repos.d/mongodb-org-7.1.repo > /dev/null <<EOF
[mongodb-org-7.0]
name=MongoDB Repository
baseurl=https://repo.mongodb.org/yum/amazon/2023/mongodb-org/7.0/x86_64/
gpgcheck=1
enabled=1
gpgkey=https://pgp.mongodb.com/server-7.0.asc
EOF

sudo dnf install -y mongodb-org mongodb-mongosh-shared-openssl3 openssl mongodb-org-database-tools-extra mongodb-database-tools mongodb-org-tools mongodb-org-server mongodb-org-mongos mongodb-org-database
sudo systemctl restart mongod


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
# and the project's node that offers the relay service.
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
# attribute mongodb-inlet="true" to connect to TCP Portal Inlets on this node.
#
# Create a TCP Portal Outlet to mongodb.
# This makes the remote mongodb available on all localhost IPs at - 0.0.0.0:17017
ockam node create
ockam relay create mongodb
ockam policy create --resource-type tcp-outlet --expression '(= subject.mongodb-inlet "true")'
ockam tcp-outlet create --to "127.0.0.1:27017"

EOS
