# File transfer

This Rust project shows how to create workers on different nodes to handle file transfers:

- `examples/receiver.rs` creates a node to receive files
  - it opens a secure channel listener
  - it creates a relay to so that files can be transferred without the node being exposed on a public network

- `examples/sender.rs` creates a node to send files
  - it connects to the relay
  - it creates a secure channel to the receiver node
  - it reads a local file, and sends it, chunk by chunk to the receiver, over the secure channel
