#!/bin/bash
set -ex

sudo bash << 'EOS'
set -ex

# Install Ockam Command
export OCKAM_VERSION="$OCKAM_VERSION"
curl --proto '=https' --tlsv1.2 -sSfL https://install.command.ockam.io | bash
source "$HOME/.ockam/env"

# PROD RENDEZVOUS
export OCKAM_RENDEZVOUS_SERVER="rendezvous.orchestrator.ockam.io:443"

ockam identity create alice

ockam project enroll "$ENROLLMENT_TICKET" --identity alice

ockam node create alice --identity alice --enable-udp
ockam tcp-inlet create --ebpf --at alice --enable-udp-puncture --disable-tcp-fallback --from 0.0.0.0:4000 --via bob

EOS
