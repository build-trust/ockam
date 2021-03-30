---
title: Rust Guide
order: 1
---

# Rust Guide

This is a guide to using the Ockam Rust SDK. Over the course of four examples, the guide builds a distributed echo service
that forwards messages between nodes.

The <a href="./get-started">Get Started</a> guide walks developers through the fundamentals of using
Ockam in a rust project.

In <a href="./01-workers">Step 1</a> we build a basic Ockam node and worker. The core data types and traits that comprise
the Ockam API are introduced, along with the asynchronous runtime.

<a href="./02-transports">Step 2</a> introduces Ockam transports and message routing. The Ockam TCP transport is used to
send messages between two nodes.

The Ockam Hub is a remote node that can send, receive, and route messages between nodes. <a href="03-hub">Step 3</a>
shows you how to use the TCP transport to send messages to a node hosted on the Hub.

Message forwarding is discussed in <a href="04-forwarding">Step 4</a>, enabling you to route messages between remote nodes.

## Guides

| Name                                                                                           | Description                                     |
| ---------------------------------------------------------------------------------------------- | ----------------------------------------------- |
|<a href="./get-started">Get Started</a>| Get ready to use the Ockam Rust SDK.|
|<a href="./01-workers">Step 1</a>| Build your first node and worker.|
|<a href="./02-transports">Step 2</a>| Send messages between nodes.|
|<a href="./03-hub">Step 3</a>| Learn how to use the Ockam Hub.|
|<a href="./04-forwarding">Step 4</a>| Use Ockam Hub to forward messages between nodes.|

# Have questions? Let us help!

**We are here to help.** See the [Guides And Demos](https://github.com/ockam-network/ockam/discussions/1134) in
GitHub Discussions.
