# Step 1 - The Ockam Runtime

## Nodes and Workers

The Ockam Rust library is comprised of many feature crates working together in an asynchronous execution environment. This environment is called an Ockam Node.

An Ockam Node manages all tasks and provides a light weight messaging system. Individual tasks in an Ockam Node are called Workers. Workers are small, concurrent message handling functions.

In a `std` environment the execution environment defaults to an implementation using `tokio`. Experimental support is available for a `no_std` async environment.

 The Node and Worker environment is similar to an actor framework. A Node manages many Workers, which pass messages between each other.

Workers have several properties of an actor system:
- unique address used for message destination
- message based communication with other Workers
- starts other Workers dynamically
- managed individually

Knowledge of actor systems is not needed to use Workers.

Workers interact with the Node by using a Context. A Context provides APIs for sending messages, starting and stopping Workers, and running tasks.

Nodes connect to other Nodes via transport protocols. Once connected, Workers can form message routes to Workers on other Nodes. Nodes use a routing protocol to forward and deliver messages.

## Creating a Node

Creating a Node is done by using the `#[ockam::node]` attribute, which wraps the Rust `main` function in an async function.

```rust
use ockam::{Context, Result};  
  
#[ockam::node]  
async fn main(mut ctx: Context) -> Result<()> {  
    ctx.stop().await
}
```

The `main` function of a Node differs from a standard Rust `main` function:
- it is `async`, allowing the use of await
- it has a parameter of `Context`
- it returns a `Result`

These three differences enable smooth use of the Ockam APIs which are largely async and return `Result`.

When you create an Ockam Node, a Worker representing your application is created automatically. The Context of this application Worker is the paramer that is passed to the `main` function.

A Node will run forever, until it is explicitly stopped. In the example above, `Context` is used to stop the Node.

## Creating a Worker

A Worker is any type that implements the `Worker` trait. This `Worker` async trait defines the lifecycle and message handling APIs for a Worker.

Async traits can be difficult to work with. The `#[ockam::worker]` trait is used to simplify Worker implementation. This attribute provides the necessary async support.

The `Worker` trait has one required function that must be implemented: `handle_message`. This function is called for each new message that arrives for a Worker.

The trait also includes functions that allow customized behavior for Worker initialization and shutdown.

When implementing the `Worker` trait, associated types must be specified for the types of `Context` and `Message`. In almost all cases, the `Context` type should be that of `ockam::Context`. This is the Context type used throughout the Node.

The `Message` type of a `Worker` defines the expected type of message that will be received by `handle_message`. Any serializable, thread safe type can be used as a Message type.

These types can be seen in the signature of `handle_message`:

```rust
async fn handle_message(  
 &mut self,  
 _context: &mut Self::Context,  
 _msg: Routed<Self::Message>,  
) -> Result<()> {  
    Ok(())  
}
```

The `Context` type is the type of the `_context` parameter, providing the Worker's API to the Node. The `Message` type is the type of `_msg`, which is also wrapped in a type that provides message routing information.

Below is a Worker implementation that prints a messaged received:

```rust
pub struct PrintWorker;  
  
#[ockam::worker]  
impl Worker for PrintWorker {  
  type Message = String;  
  type Context = Context;  
  
  async fn handle_message(  
    &mut self,  
    ctx: &mut Self::Context,  
    msg: Routed<Self::Message>,  
  ) -> Result<()> {  
     println!("{}", msg);  
     ctx.stop().await  
  }  
}
```

## Starting a Worker

A Node Context is used to start Workers. The Context function `start_worker` takes two parameters: the address of the Worker, and an implementation of a Worker. After the Worker is started, we also use the Context to send the new Worker a message.

```rust
#[ockam::node]  
async fn main(mut ctx: Context) -> Result<()> {  
    ctx.start_worker("printer", PrintWorker {}).await?;  
    ctx.send("printer", "Hello, world!".to_string()).await  
}
```


## Demo Source

```rust
use ockam::{Context, Result, Routed, Worker};  
  
pub struct PrintWorker;  
  
#[ockam::worker]  
impl Worker for PrintWorker {  
    type Message = String;  
    type Context = Context;  
  
    async fn handle_message(  
        &mut self,  
        ctx: &mut Self::Context,  
        msg: Routed<Self::Message>,  
    ) -> Result<()> {  
        println!("{}", msg);  
        ctx.stop().await  
    }  
}  
  
#[ockam::node]  
async fn main(mut ctx: Context) -> Result<()> {  
    ctx.start_worker("printer", PrintWorker {}).await?;  
    ctx.send("printer", "Hello, world!".to_string()).await  
}
```

## Next Steps

Workers can send messages to remote Workers on other Nodes. In Step 2, Nodes will connect to other Nodes using a transport, and messages will be sent between them using routing.
