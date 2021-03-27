---
title: Build a Node
order: 2
---

# Nodes and Workers

This example introduces the concepts of Ockam nodes and workers.

## Getting Started

Create a new Rust binary with cargo:

```shell
cargo new worker
```

Add the `ockam` and `ockam_node` dependencies to your project:

```toml
[dependencies]
ockam = "0"
ockam_node = "0"
```

## Ockam Workers

Add the Ockam import statements to your `main.rs`

```rust
use ockam::{async_worker, Context, Result, Worker};
```

An Ockam Worker is any struct that implements the `Worker` trait. Workers have two associated types, which represent the
kind of messages the worker processes, and the API that is available when a message arrives. These associated types are
called the `Message` Type and the `Context` Type. Most Ockam Node implementations should use the default `Context` type.
The `Message` type is specific to the worker implementation.

The `Worker` trait is an async trait. Rust requires some additional support to use traits which have async methods. To
make writing workers simpler, the ockam `#[async_worker]` attribute is used. It is important to note that since the Ockam
APIs use Rust lazy async/await, work begins only when await is called.

In this example we create a worker that has a Message type of `String`. When the worker receives a message, it responds
with the same message.

```rust
struct Echoer;

#[async_worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: String) -> Result<()> {
        ctx.send_message("app", format!("{}", msg)).await
    }
}
```

## Ockam Node

An Ockam node is an asynchronous environment. Ockam provides an attribute for your main function to help you get started
quickly. This attribute is called `#[ockam::node]`. This attribute has several properties:

- An `async fn main` function wraps the real `main` function, for compatibility with the rest of the system. You may see this wrapper in stack traces.
- An input parameter to `main` of type `Context` provides the Ockam Node API to your application.
- `main` returns an `ockam_core::Result`. This allows for better error handling when using Ockam APIs.

Putting all of this together, we arrive at a node `main` function:

```rust
#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    /// Awesome code
}
```

## Starting a Worker

To start a worker, use the `Context` trait provided to our `main` function. The `Context` trait provides a `start_worker`
API that starts your worker at a given Address:

```rust
ctx.start_worker("echoer", Echoer).await?;
```

## Sending a message to a Worker

Workers respond to messages. The `Context` trait is used to send messages:

```rust
ctx.send_message("echoer", "Hello Ockam!".to_string()).await?;
```

## Receiving a reply message

Workers also listen on their addresses for incoming messages. Incoming messages can be retrieved with the `Context` trait:

```rust
let reply = ctx.receive::<String>().await?;
println!("Reply: {}", reply);
```

## Stopping the Node

The Ockam Node can be stopped by calling the `Context` trait `stop` API.

```rust
ctx.stop().await
```

## Putting it all together

The API calls above implement the `main` method of the worker. Here is the complete worker implementation:

```rust
use ockam::{async_worker, Context, Result, Worker};

struct Echoer;

#[async_worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: String) -> Result<()> {
        ctx.send_message("app", format!("{}", msg)).await
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    ctx.start_worker("echoer", Echoer).await?;

    ctx.send_message("echoer", "Hello Ockam!".to_string()).await?;

    let reply = ctx.receive::<String>().await?;
    println!("Reply: {}", reply);

    ctx.stop().await
}
```

## Running the example

From the `ockam_examples` crate, run:

```shell
cargo run --example guide_01_workers
```
