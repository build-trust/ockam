# Ockam worker architecture

The following tree of documents outlines the basic design of the ockam
worker architecture, integration into different nodes, and how to
write code against the API to run as workers.

The basic architecture of workers is modelled quite closely on [actor
programming], inspired by languages such as [Erlang], and in part the
Rust library [actix].

[actor programming]: https://en.wikipedia.org/wiki/Actor_model
[Erlang]: https://www.erlang.org/
[actix]: https://github.com/actix/actix

Workers are managed by a supervisor task that will restart them if an
error occurs.  Generally they go through the following steps.

1. Initialization (calls `Worker::initialize(...)`)
2. Handle messages (calls `Worker::handle_message(...)` per message)
3. Blocked message receival (Wait for next message via `receive()`)
4. Shutdown (calls `Worker::shutdown(...)`)

Workers are created and managed via an ockam node (provided by
`ockam_node`). A worker can register a typed
message it responds to, and a handle function which is called every
time a message is sent to that particular worker address.

```rust
use ockam_node::Context;
use ockam::{Address, Message, Worker};

struct MyWorker;

#[derive(Debug, Message)]
struct enum MyMessage {
    Ping(Address),
    Stop,
}

impl Worker for MyWorker {
    type Context = Context;
    type Message = MyMessage;

    fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        println!("Starting worker {}", ctx.address())
        Ok(())
    }
}
```

Handling messages can either be done by providing a custom
`handle(...)` implementation, or calling `receive()` on the node.

```rust
impl Worker for MyWorker {
    // ...

    fn handle_message(&mut self, _: &mut Context, msg: Self::Message) -> Result<()> {
        println!("Message received: {:?}", msg)
        Ok(())
    }
}
```
