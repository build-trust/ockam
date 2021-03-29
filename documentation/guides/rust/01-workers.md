---
title: Workers
order: 2
---

# Workers

## Creating a Node

The Ockam API provides an asynchronous message passing runtime called a [Node](https://docs.rs/ockam_node/0.4.0/ockam_node/).
Bootstrapping a node is done like a normal rust program, with a slightly different `main` function. The attribute [#[ockam::node]](https://docs.rs/ockam_node_attribute/0.1.4/ockam_node_attribute/) hides
much of this runtime.

The `#[ockam::node]` attribute has several properties:

- An `async fn main` function wraps the real `main` function, for compatibility with the rest of the system. You may see this wrapper in stack traces.
- An input parameter to `main` of type `Context` provides the Ockam Node API to your application.
- `main` returns an [ockam::Result](https://docs.rs/ockam/0.4.0/ockam/type.Result.html). This allows for better error handling when using Ockam APIs.

Below is a minimal node which only stops itself. If the `stop` API call is omitted, the node will continue to poll for messages.

```rust
#[ockam::node]
async fn main(context: ockam::Context) {
    context.stop().await.unwrap();
}
```

[Context](https://docs.rs/ockam/0.4.0/ockam/struct.Context.html) is an essential API when writing a node application. This
context provides access to the message passing and node life-cycle functions, among other things.

## Messaging and Workers

Ockam Workers are message handling entities. When a message with a worker's address is received on a Node, that message
is forwarded to the Worker. The worker's message handling function - [handle_message](https://docs.rs/ockam_core/0.6.0/ockam_core/trait.Worker.html#method.handle_message) - is called.

Workers can also send messages using the Context [send_message](https://docs.rs/ockam_node/0.4.0/ockam_node/struct.Context.html#method.send_message) API.

## Creating a Worker

An Ockam Worker is any struct that implements the `Worker` trait. Workers have two associated types, which represent the
kind of messages the worker processes, and the API that is available when a message arrives. These associated types are
called the `Message` Type and the `Context` Type. Most Ockam Node implementations should use the default `Context` type.
The `Message` type is specific to the worker implementation.

The `Worker` trait is an async trait. Rust requires some additional support to use traits which have async methods. To
make writing workers simpler, the ockam [#[async_worker]](https://docs.rs/ockam/0.4.0/ockam/attr.async_worker.html) attribute is used. It is important to note that since the Ockam
APIs use Rust lazy async/await, work begins only when await is called.

In this example we create a worker that has a Message type of `String`. When the worker receives a message, it responds
with the same message.

```rust
use ockam::{async_worker, Context, Result, Worker};

struct Echoer;

#[async_worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;
}
```

## Registering and Starting a Worker

Workers must be registered on a Node in order to receive messages. This is done using the Context [start_worker](https://docs.rs/ockam_node/0.4.0/ockam_node/struct.Context.html#method.start_worker)
API. This function starts a worker on the given address.

```rust
ctx.start_worker("echoer", Echoer).await?;
```

Once the node processes the worker's registration, the [initialize](https://docs.rs/ockam_core/0.6.0/ockam_core/trait.Worker.html#method.initialize)
handler of the Worker will be invoked. This is an ideal place to add additional setup code for your worker.

If you don't need to perform any special steps during Worker startup, you can omit the `initialize` function and use the
default of no operation.

## Handling Messages

Workers have message handling callbacks that are invoked when a new message arrives for the Worker's address.

The Echo worker being built will take the incoming message, and use the `send_message` API to echo the message back to
the sender.

An updated Echo worker now looks like this:

```rust
#[async_worker]
impl Worker for Echoer {
    type Message = String;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: String) -> Result<()> {
        ctx.send_message("app", format!("{}", msg)).await
    }
}
```

## Receiving a Message

There are two ways to receive a message as a Worker:
- Wait for the node to call `handle_message`. This is the typical scenario.
- Use the `Context` API to block on a call to [receive](https://docs.rs/ockam/0.4.0/ockam/struct.Context.html#method.receive).
This function will block the current thread until a message is available.

In the complete example, both ways of receiving messages are demonstrated.

## Stopping the Node

The Ockam Node can be stopped by calling the `Context` trait `stop` API.

```rust
ctx.stop().await
```

# Putting it all together - Echo Worker

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
