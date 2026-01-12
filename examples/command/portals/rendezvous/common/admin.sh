#!/bin/bash
set -ex

# Change into ec2-user's home directory and use sudo to run the commands as ec2-user
sudo bash << 'EOS'
set -ex

# Install Ockam Command
export OCKAM_VERSION="$OCKAM_VERSION"
curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
source "$HOME/.ockam/env"

# PROD RENDEZVOUS
export OCKAM_RENDEZVOUS_SERVER="rendezvous.orchestrator.ockam.io:443"

ockam identity create bob

ockam project enroll "$ENROLLMENT_TICKET" --identity bob

python3 -m http.server --bind 127.0.0.1 8000 &

ockam node create bob --identity bob --enable-udp
ockam tcp-outlet create --ebpf --at bob --to 127.0.0.1:8000
ockam relay create bob --to /node/bob

EOS
