# UDP Hole Punching Setup

This directory contains examples of how UDP hole punching might be
achieved using the Ockam Elixir UDP transport and simple workers.

## Overview

UDP hole punching is a technique used to establish a direct communication channel between two devices that are behind NAT (Network Address Translation) routers. This is achieved by sending UDP packets to a third-party rendezvous server that provides
each node with the other's external facing address.

## Usage

- Start the rendezvous server
```
elixir 01-rendezvous.exs
```

- Start two (or more) clients (punchers)
```
elixir 01-puncher.exs my_name their_name rendezvous_host:port
elixir 01-puncher.exs my_name their_name rendezvous_host:port
```

## Known Issues

- Generally unresilient
    - doesn't handle address changes
    - doesn't implement keep alives
    - doesn't handle multi hop
- Hard coded rendezvous node name
- Identifiers are just simple strings
- Uses underlying Elixir UDP transport which is currently incompatible with the Rust impl


