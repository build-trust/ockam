# Message data format

Ockam Routing protocol defines payload as opaque data sent with messages.

Payload is used primarily to carry application data exchange.

## Data and metadata

Ockam Messaging describes workers behaviour mostly using routes,
but also introduces a concept of Message Metadata.

Metadata is a portion of a message which is not a part of application data,
but used to facilitate delivery.

For example, in indexed pipes each message is assigned an index.
While message data is preserved, message is extended with index metadata.

Part of the message which carries the application data we call **Data portion** and the part which carries delivery specific information we call **Metadata portion**.

Not all messages carry data portion, some messages facilitate some internal communication between workers, like handshakes or confirmations. Those messages only have metadata portion.

## Metadata scopes

Ockam Messaging describes delivery as a combination of multiple end-to-end deliveries which can be combined with each other by [pipelining or wrapping](./messaging/Delivery.md#core-combination-techniques)

Each delivery defines its own behaviour and its own internal messaging protocol. This means that even in a single message delivery each component will have its own metadata format.

This makes designing generic metadata format challenging because

- metadata keys can overlap
- metadata of the external message should be preserved in the internal delviery
- deliveries may use different encoding format for metadata

We can define a data format at any point in delivery, and we can define the points when data gets transformed between formats.

This way we can define a **Scope** in which message uses a certain data format.

We can then organize the Scopes in a way that allows us to reason about and access message metadata at a certain point in delivery.

More info on metadata scopes in [Scopes](./Scopes.md)

### Scope agnostic metadata

Some message metadata, like tracing information could be useful across the scopes to create a consistent trace. There is a case for **cross-scope metadata**.

In order to distinguish scope agnostic metadata and simplify usage of metadata scopes, we can use the payload format proposed in [Payload Format](./Payload.md)

## Implementing multiple data formats in workers

Because each scope has its own data format, the worker which send and receive messages in a certain scope can use scope specific data encoding as long as they can validate that the message is actually using expected scope format.

Workers which transform messages between multiple scopes may use [different addresses](../messaging/Routing.md#workers-with-multiple-addresses) and assign a single scope to each address for convenience.

This way each address would handle messages in only one data format.

More on implementing multiple scope workers in [Implementation](./Implementation.md)




