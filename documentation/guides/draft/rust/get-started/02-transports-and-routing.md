# Step 2 - Transports and Routing

## Transports

Ockam Transports are logical connections between Ockam Nodes. Ockam Transports are an abstraction on top of physical transport protocols.

The Ockam TCP Transport is an implementation of an Ockam Transport using the TCP protocol. This functionality is available in the `ockam_transport_tcp` crate, and is included in the standard feature set of the top level `ockam` crate.

## Using the TCP Transport

The Ockam TCP Transport API fundamental type is `TcpTransport`. This type provides the ability to create, connect, and listen for TCP connections.

To create a TCP transport, the Context is passed to the `create` function:

```rust
let tcp = TcpTransport::create(&ctx).await
```

The return value of `create` is a handle to the transport itself, which is used for `connect` and `listen` calls.

Listening on a local port is accomplished by using the `listen` method. This method takes a string containing the IP address and port, delimited by `:`. For example, this statement will listen on localhost port 3000:

```rust
tcp.listen("127.0.0.1:3000").await
```

The `connect` API function has the same signature as `listen`. The `IP:port` argument is the remote peer information used to create a TCP connection.

```rust
tcp.connect("127.0.0.1:3000").await
```

The `connect` and `listen` APIs only need to be called once to establish a Transport connection. Messages are automatically sent over a connected transport when required by a route.

## Message Routing

A route is a list of worker addresses. The order of addresses in a route defines the path a message will take from its source worker to its destination worker.

A message has two routes: the **onward route** and the **return route**.

The forward route specifies the path the message takes to the destination. When a node receives a message to route, the head of the address list is removed from the route. This address is used to determine the next destination route, or destination worker.

The return route of a message represents the path back to the source worker. The return route may differ from the onward route. When a message is routed through a node, the node adds its own address to the return route. This ensures that there is a valid, known return path for message replies.

All messages sent in an Ockam Network have a route. Many messages between localworkers have short routes, only indicating the address of another local Worker.

Routes can be constructed in several ways:

- using the `route!` macro
- as a `Vec` or `Array` of strings or addresses
- a single string or address

```rust
let route = route!["a", "b"];  
let route: Route = "a".into();  
let route: Route = vec!["a", "b"].into();
```

Most APIs that require a `Route` can also use any type that can be turned into a `Route`.

## Routing over Transports

Transports are implemented as workers, and have a unique address. The transport address is used in routes to indicate that the message must be routed to the remote peer. 

Transport addresses also encode a unique protocol identifier. This identifier is prefixed to the beginning of an address, followed by a `#`. The portion of an address after the `#` is transport protocol specific.

The TCP transport has a transport protocol identifier of `1`, which is also aliased to the constant `TCP`. The actual address uses the familiar `IP:PORT` format. A complete TCP transport address could appear such as `1#127.0.0.1:3000`.

Transport addresses can be created easily using the tuple syntax:

```rust
// Creating an address using a transport identifier, tuple and into
let remote : Address = (TCP, "127.0.0.1:3000").into();

// Implicit conversion from tuple to address
let route = route![(TCP, "10.0.0.1:8000")];
```

To send a message to a worker on another node connected by a transport, the address of the transport is added to the route first, followed by the address of the destination worker.

```rust
// This route forwards a message to the remote TCP peer Node
// and then to Worker "b"
let route = route![(TCP, "127.0.0.1:3000"), "b"]
```


## Example: Hop and Echoer

This example demonstrates message routing between nodes over a transport.

In this example, we will create three nodes:

- **Echoer**: Replies to any message with a copy of the same message
- **Hop**: An intermediate Node that routes messages
- **App**: Sends a message to Echoer through Hop and receives a reply


### Hop Source

### Echoer Source

### App Source
