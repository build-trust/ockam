---
title: Rust Guide
order: 1
---

# Rust Guide

This is a guide to using the Ockam Rust SDK. Over the course of four examples, the guide builds a distributed echo service
that forwards messages between nodes.

The [Get Started](get-started) guide walks developers through the fundamentals of using
Ockam in a rust project.

In [Step 1](01-workers) we build a basic Ockam node and worker. The core data types and traits that comprise
the Ockam API are introduced, along with the asynchronous runtime.

[Step 2](02-transports) introduces Ockam transports and message routing. The Ockam TCP transport is used to
send messages between two nodes.

The Ockam Hub is a remote node that can send, receive, and route messages between nodes. [Step 3](03-hub)
shows you how to use the TCP transport to send messages to a node hosted on the Hub.

Message forwarding is discussed in [Step 4](04-forwarding), enabling you to route messages between remote nodes.

## Guides

| Name                                                                                           | Description                                     |
| ---------------------------------------------------------------------------------------------- | ----------------------------------------------- |
|[Get Started](get-started)| Get ready to use the Ockam Rust SDK.|
|[Step 1](01-workers)| Build your first node and worker.|
|[Step 2](02-transports)| Send messages between nodes.|
|[Step 3](03-hub)| Learn how to use the Ockam Hub.|
|[Step 4](04-forwarding)| Use Ockam Hub to forward messages between nodes.|

# Have questions? Let us help!

**We are here to help.** See the [Guides And Demos](https://github.com/ockam-network/ockam/discussions/1134) in
GitHub Discussions.
