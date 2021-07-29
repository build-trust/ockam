```yaml
title: Get Started
```

## Get started

In this step-by-step guide we’ll show code examples that exchange end-to-end
encrypted messages. We'll introduce various Ockam features that enable secure
communication between distributed applications.

<div style="display: none; visibility: hidden;"><hr></div>

## Setup

1. Install Rust

`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

2. Setup a hello_ockam Cargo Project to get started with Ockam

`cargo new --lib hello_ockam && cd hello_ockam && mkdir examples && echo 'ockam = "*"' >> Cargo.toml && cargo build`

For more details on the setup process, see the [Setup Step](./00-setup).

<ul>
<li><a href="./01-node">01. Node</a></li>
<li><a href="./02-worker">02. Worker</a>
<li><a href="./03-routing">03. Routing</a></li>
<li><a href="./04-transports">04. Transports</a></li>
<li><a href="./05-entities">05. Entities</a></li>
<li><a href="./06-secure-channels">06. Secure Channels</a></li>
<li><a href="./xx-hub-node">XX. Hub Nodes</a></li>
<li><a href="./xx-connecting-devices-using-hub-node">XX. Connecting devices using Hub Nodes</a></li>
<li><a href="./xx-secure-channel-over-hub-node">XX. Secure Channel over Hub Nodes</a></li>
</ul>
