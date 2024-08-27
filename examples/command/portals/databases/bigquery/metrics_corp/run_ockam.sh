#!/bin/bash
set -ex

# Change into ec2-user's home directory and use sudo to run the commands as user
cd ~
sudo bash << 'EOS'
set -ex

# Create a reverse proxy to the BigQuery API
cat << 'EOF' > proxy.js
const http = require('http');
const httpProxy = require('http-proxy');

// Create a proxy server to forward requests to the BigQuery API
const proxy = httpProxy.createProxyServer({
    target: 'https://bigquery.googleapis.com',
    changeOrigin: true,
});

const server = http.createServer((req, res) => {
    proxy.web(req, res, (err) => {
        if (err) {
            console.error(`Error proxying request: ${err.message}`);
            res.writeHead(502, { 'Content-Type': 'text/plain' });
            res.end('Bad Gateway');
        }
    });
});

const port = 8000;
server.listen(port, () => {
    console.log(`Reverse proxy server listening on port ${port}`);
});
EOF

# Install Node.js and start the reverse proxy at the background
sudo apt update -y && sudo apt install nodejs npm -y
npm install http-proxy
node proxy.js &

# Install Ockam Command
export OCKAM_VERSION="$OCKAM_VERSION"
curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
source "$HOME/.ockam/env"

ockam --version

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
# Create an encrypted relay to the reverse proxy, ensuring requests made through the relay is received by the proxy.
# The relay makes this node reachable by other project members.
#
# Create an access control policy that only allows project members that possesses a credential with
# attribute bigquery-inlet="true" to connect to TCP Portal Outlets on this node.
#
# Create a TCP Portal Outlet to our reverse proxy
cat << EOF > outlet.yaml
tcp-outlet:
  to: "127.0.0.1:8000"
  allow: '(= subject.bigquery-inlet "true")'

relay: bigquery
EOF

ockam node create outlet.yaml
rm outlet.yaml

EOS
