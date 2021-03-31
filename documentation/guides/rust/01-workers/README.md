---
title: Workers
order: 3
---

#### Have questions? Let us help!

**We are here to help.** See the [Guides And Demos](https://github.com/ockam-network/ockam/discussions/1134) in
GitHub Discussions.

# Nodes and workers

Ockam provides an asynchronous execution environment called a node. The node routes messages to individual workers. Workers
are similar to actors. They receive messages and can perform actions in response.

When a message with a worker's address is received on a node, the message is forwarded to the worker. The worker's `handle_message`
function is called with the message as a parameter.

Workers can send messages using the `Context::send_message` API.

## Create a node

The Ockam API provides an asynchronous message passing runtime called a node.
Bootstrapping a node is done like a normal rust program, with a slightly different `main` function. The attribute `#[ockam::node]` hides
much of this runtime.

The `#[ockam::node]` attribute has several properties:

- The real `main` function wraps an `async fn main` function, for compatibility with the rest of the system. You will see
  this wrapper in stack traces as two 'main' functions.
- An input parameter to `main` of type `Context` provides the message passing, node, and other Ockam APIs.

- `main` returns an `ockam::Result`. This allows for better error handling when using Ockam APIs.

Below is a minimal node which only stops itself. If the `stop` API call is omitted, the node will continue to poll for messages.

```rust
#[ockam::node]
async fn main(context: ockam::Context) -> ockam::Result<()> {
    context.stop().await
}
```

## Create a worker

An Ockam Worker is any struct that implements the `Worker` trait. Workers have two associated types, which represent the
kind of messages the worker processes, and the API that is available when a message arrives. These associated types are
called the `Message` Type and the `Context` Type. Most Ockam Node implementations should use the default `Context` type.
The `Message` type is specific to the worker implementation.

The `Worker` trait is an async trait. Rust requires some additional support to use traits which have async methods. To
make writing workers simpler, the ockam `#[async_worker]` attribute is used. It is important to note that since the Ockam
APIs use Rust lazy async/await, work begins only when await is called.

In this example we create a worker that handles messages of a `String` type. When the worker receives a message, it responds
with the same message.

```rust
use ockam::{async_worker, Context, Result, Worker, Routed};

struct EchoService;

#[async_worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("echo_service: {}", msg);
        ctx.send_message(msg.reply(), msg.take()).await
    }
}
```

# Start the worker

Workers must be registered on a node in order to receive messages. This is done using the `Context::start_worker` API.
This function starts a worker on the given address.

```rust
ctx.start_worker("echo_service", EchoService).await?;
```

Once the node processes the worker's registration, the `initialize` handler of the `Worker` is invoked. This is an ideal
place to add additional setup code.

If you don't need to perform any special steps during startup, you can omit the `initialize` function and use the default of no operation.

## Send a message

The `echo_service` takes the incoming message and use the `send_message` API to echo the message back to
the sender.

```rust
ctx.send_message(msg.reply(), msg.take()).await?;
```

Likewise, the app sends the initial message to the `echo_service` using this API:

```rust
ctx.send_message("echo_service", "Hello Ockam!".to_string()).await?;
```

## Receive a message

Workers have message handling callbacks that are invoked when a new message arrives for the worker's address.

There are two ways to receive a message as a worker:

- Wait for the node to call `handle_message`. This is the typical scenario.
- Use the `Context` API to block on a call to `receive`. This function will block the worker until a message is available.

```rust
let reply = ctx.receive::<String>().await?;
```

## Stop the node

The Ockam Node can be stopped by calling the `Context` trait `stop` API.

```rust
ctx.stop().await?;
```

# Putting it all together - Echo Service

```rust
use ockam::{async_worker, Context, Result, Routed, Worker};

struct EchoService;

#[async_worker]
impl Worker for EchoService {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<String>) -> Result<()> {
        println!("echo_service: {}", msg);
        ctx.send_message(msg.reply(), msg.take()).await
    }
}

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    ctx.start_worker("echo_service", EchoService).await?;

    ctx.send_message("echo_service", "Hello Ockam!".to_string())
        .await?;

    let reply = ctx.receive::<String>().await?;
    println!("Reply: {}", reply);

    ctx.stop().await
}

```

Run the example:

```shell
cargo run --example echo_service
```

Now we are ready to [use a transport](../02-transports) to connect to remote nodes.
