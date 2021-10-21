# balenaBlock Ockam

---


Ockam is a set of libraries for end-to-end encrypted, mutually authenticated,
secure communication.

## Features

The Ockam balenaBlock creates secure access tunnels to remote services and devices that are running in a private network, behind a NAT.

This block functions as an Ockam TCP Inlet and Outlet pair.

A TCP Outlet starts up as a TCP client to a given target TCP server address. It opens a TCP connection with the target and then waits to receive Ockam Routing messages from an Inlet. The payload of an incoming Ockam Routing message is converted into raw TCP and sent over the outlet connection to the target. Any replies from the TCP target are wrapped as payloads of new Ockam Routing messages and routed to the Inlet.

A TCP Inlet starts up as a TCP server and waits for incoming TCP connections. It knows the route to a corresponding outlet. When new data arrives from a TCP client, the Inlet wraps this data as payload of a new Ockam Routing message and sends it to the Outlet.

## Configuration

---

There are four environment variables that control the block.

- `OCKAM_HUB` - Host and port of an Ockam Hub Node. Example: `1.node.hub.network:4000`
- `OCKAM_IN` - Host and port for inlet listener. Enables inlet mode. Example: `127.0.0.1:5000`
- `OCKAM_FORWARD` - Ockam forwarding address of an outlet. Required for inlet mode. Example: `abcdef01234`
- `OCKAM_OUT` - Host and port for outlet connection. Enables outlet mode. Example: `10.10.10.10:8080`

A block running in inlet mode, the common case, requires the `OCKAM_HUB`, `OCKAM_FORWARD`, an `OCKAM_IN` environment variables to be set.

A block running in outlet mode requires the `OCKAM_HUB` and `OCKAM_OUT` environment varibles to be set.

A block cannot run in both inlet mode and outlet mode. Inlet mode takes precedent.

## Usage

Add the block to your docker-compose file as a new service. Static environment variables can be
configured in the `environment` block. Variables such as `OCKAM_FORWARD` which are available at runtime
should be configured using Fleet Variables.

---

### docker-compose file

```yaml
  balena_ockam:
    build:
      context: ./balena_ockam
    network_mode: host
    environment:
      - OCKAM_IN=127.0.0.1:5000               # Example inlet.
      - OCKAM_HUB=1.node.ockam.network:4000   # Example hub.
```

### Example Outlet

Running the block in outlet mode:

```bash
OCKAM_HUB=1.node.ockam.network:4000 OCKAM_OUT=127.0.0.1:8080 ./balena_ockam
```

Produces output such as:

```
INFO ockam_vault::software_vault: Creating vault
INFO ockam_channel::secure_channel: Starting SecureChannel listener at 0#cbaf6f03e7f92f343810445a1db48cf1
INFO ockam::remote_forwarder: RemoteForwarder registered with route: 1#50.18.236.180:4000 => 0#f54140e3d74ca1c3
Started Ockam Outlet at forwarding address f54140e3d74ca1c3 on 1.node.ockam.network:4000 output to 127.0.0.1:8080
```

In this case, the forwarding address `f54140e3d74ca1c3` is the value to be used as `OCKAM_FORWARD` for inlet blocks.


## Supported Devices

---

- Raspberry Pi 3b+
- Intel NUC

