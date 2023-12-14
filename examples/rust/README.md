# Examples

This directory contains some Rust projects which are examples of what can be achieved with Ockam:

1. `get_started`: step-by-step examples showcasing routing, transports, secure channels, credentials
2. `file_transfer`: shows how to transfer files of various size over a secure channel
3. `tcp_inlet_and_outlet`: shows how to create a secure portal to a remote service
4. `kafka`: shows how Kafka publishers and consumers can exchange data over secure channels
5. `no_std`: starter project for using the Ockam Rust library in a `no_std` configuration (typically used for small devices)
6. `mitm_node`: shows how the system is resistant to a man-in-the-middle attack, with a node that would be able to intercept TCP connections
