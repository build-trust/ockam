#!/bin/bash
set -ex

# Change into ec2-user's home directory and use sudo to run the commands as ec2-user
cd /home/ec2-user
sudo -u ec2-user bash << 'EOS'
set -ex

# Install InfluxDB client
curl -OL# https://download.influxdata.com/influxdb/releases/influxdb2-client-2.7.5-linux-amd64.tar.gz
tar xvf influxdb2-client-2.7.5-linux-amd64.tar.gz

# Configure the client and wait until it can connect with the database
./influx config create --active --config-name test --org "metrics_corp" --username-password "admin:YourSecurePassword" \
    --host-url "https://$INFLUXDB_ADDRESS:8086"
while ! ./influx bucket list; do sleep 10; done

# Create a new bucket to store metrics
./influx bucket create --name metrics

# Generate an influxdb auth token
./influx auth create --all-access --json | jq -r .token > token.txt

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
# The authority enrolls presented identity and returns a project membership credential.
#
# The command, stores this credential for later use and exits.
ockam project enroll "$ENROLLMENT_TICKET"

# Create an ockam node.
#
# Create an encrypted relay to this node in the project at address: influxdb.
# The relay makes this node reachable by other project members.
#
# Create an access control policy that only allows project members that possesses a credential with
# attribute influxdb-inlet="true" to connect to TCP Portal Outlets on this node.
#
# Create a TCP Portal Outlet to influxdb at - $INFLUXDB_ADDRESS:8086.
cat << EOF > outlet.yaml
tcp-outlet:
  to: "$INFLUXDB_ADDRESS:8086"
  allow: '(= subject.influxdb-inlet "true")'
relay: influxdb
EOF
ockam node create outlet.yaml
rm outlet.yaml

EOS
