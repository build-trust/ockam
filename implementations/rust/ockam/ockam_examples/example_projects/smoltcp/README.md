# Example using [smoltcp](https://github.com/smoltcp-rs/smoltcp)

## Description

[Smoltcp](https://github.com/smoltcp-rs/smoltcp) is a library that allows to run a TCP stack without depending on the OS implementations()

This example uses a tap interface to demonstrate how to use the smoltcp transport.

Both the client and server examples use the same `tap0` interface so they can't be ran together, however they are designed to run along the sibling example `tcp` server and client, respectively.

_TODO: Create a separate example that connects 2 `tap` interface_

_TODO: Add an example with a hardware interface_

## Preparation

Create the tap interface:

```sh
sudo ip tuntap add name tap0 mode tap user $USER
sudo ip link set tap0 up
sudo ip addr add 192.168.69.100/24 dev tap0
sudo ip -6 addr add fe80::100/64 dev tap0
sudo ip -6 addr add fdaa::100/64 dev tap0
sudo ip -6 route add fe80::/64 dev tap0
sudo ip -6 route add fdaa::/64 dev tap0
```

## Run the server example

In a terminal in this directory($PWD) run:

```sh
cargo run --example network_echo_server
```

In another terminal on the tcp example directory($PWD/../tcp) run:

```sh
cargo run --example network_echo_client 192.168.69.1:10222
```

## Run the client example

In a terminal on the tcp example directory($PWD/../tcp) run:

```sh
cargo run --example network_echo_server 0.0.0.0:10222
```

In another terminal in this directory($PWD) run:

```sh
cargo run --example network_echo_client 192.168.69.100:10222
```

## Note

To see what's happening you want to set the logging level when running any of this example to `INFO` or `TRACE` to do that set the env variable `OCKAM_LOG` to `info` or `trace` respectively.
