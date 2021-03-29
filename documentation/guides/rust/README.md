---
title: Rust Guide
order: 1
---

# Rust Guide

This is a guide to using the Ockam Rust SDK. Over the course of four examples, the guide builds a distributeds echo service
that forwards messages between nodes.

The [Getting Started](/learn/how-to-guides/rust/getting-started) guide walks developers through the fundamentals of using
Ockam in a rust project.

In [Step 1](/learn/how-to-guides/rust/01-workers) we build a basic Ockam node and worker. The core data types and traits
that comprise the Ockam API are introduced, along with the asynchronous runtime.

[Step 2](/learn/how-to-guides/rust/02-transports) introduces Ockam transports and message routing. The Ockam TCP transport is
used to send messages between two nodes.

The Ockam Hub is a remote node that can send, receive, and forward messages between workers. [Step 3](/learn/how-to-guides/rust/03-hub)
shows you how to use the TCP transport to send messages to a hub.

Message forwarding is discussed in [Step 4](/learn/how-to-guides/rust/04-forwarding), enabling you to send messages between
Ockam nodes and the hub.

## Guides

| Name                                                                                           | Description                                     |
| ---------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| [Getting Started](/learn/how-to-guides/rust/getting-started)                 | Get ready to use the Ockam Rust SDK.   |
| [Step 1 - Workers](/learn/how-to-guides/rust/01-workers)              | Build your first Ockam Node and Worker.          |
| [Step 2 - Transports](/learn/how-to-guides/rust/02-transports)                 | Send messages between Ockam Nodes.               |
| [Step 3 - Ockam Hub](/learn/how-to-guides/rust/03-hub)                | Learn how to use the Ockam Hub.                  |
| [Step 4 - Message Forwarding](/learn/how-to-guides/rust/04-forwarding) | Use Ockam Hub to forward messages between Nodes. |
