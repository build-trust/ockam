---
title: Rust Guide
order: 1
---

# Rust Guide

This is a guide to using the Ockam Rust SDK. Over the course of four examples, the guide builds a distributed echo service
that forwards messages between nodes.

The <a href="getting-started">Getting Started</a> guide walks developers through the fundamentals of using
Ockam in a rust project.

In <a href="01-workers">Step 1</a> we build a basic Ockam node and worker. The core data types and traits
that comprise the Ockam API are introduced, along with the asynchronous runtime.

<a href="02-transports">Step 2</a> introduces Ockam transports and message routing. The Ockam TCP transport is
used to send messages between two nodes.

The Ockam Hub is a remote node that can send, receive, and forward messages between workers. <a href="03-hub">Step 3</a>
shows you how to use the TCP transport to send messages to a hub.

Message forwarding is discussed in <a href="04-forwarding">Step 4</a>, enabling you to send messages between
Ockam nodes and the hub.

## Guides

| Name                                                                                           | Description                                     |
| ---------------------------------------------------------------------------------------------- | ----------------------------------------------- |
|<a href="getting-started">Getting Started</a>| Get ready to use the Ockam Rust SDK.|
|<a href="01-workers">Step 1</a>| Build your first Ockam Node and Worker.|
|<a href="02-transports">Step 2</a>| Send messages between Ockam Nodes.|
|<a href="03-hub">Step 3</a>| Learn how to use the Ockam Hub.|
|<a href="04-forwarding">Step 4</a>| Use Ockam Hub to forward messages between Nodes.|
