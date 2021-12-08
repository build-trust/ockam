# Message data scopes

Let's say we have message exchange between application `A` and `B`
over some channel `C1, C2`

Application `A` would send messages to `OR: A->C1->B`, while `B` will receive messages
from `RR: A->C2->B`

**TODO: pictures**

Application code is aware of how messages are handled on `A->C1` and `C2->B`.

Message payload is treated by `C1,C2` as an opaque binary data.

Actual data layout is known only to `A` and `B`.
They can use some encoding protocol to encode application data
as binary and pass it to Ockam Messaging.

The application code is aware of the message protocol,
the fact that there is `A` or `B` and that there is a channel `C1,C2`.

Channel workers `C1` and `C2` are sending messages to each other,
they may use some additional message routes to get messages from `C1` to `C2`
and may use some additional metadata or message format between `C1` and `C2`

This means that channel (or pipe) workers to forward messages need to extend
the payload ot re-code the payload (e.g. secure channel encryption)

This way, messages on `A->C1` and `C2->B` would have one data format,
while messages on `C1->C2` would have another format.

We can say that messages on `A->C1` and `C2->B` exist on a different **Scope** from messages on `C1->C2`.

Any additional messages sent between `C1` and `C2` (e.g. handshakes or confirmations) exist only in the Channel Scope. Those messages data format and routes are internal to the channel.

**TODO: message format picture**

## Scope isolation

Since we might use multiple scopes in a pipelined delivery it's important to keep any scope-specific data in this scope only.

For example if one channel endpoint assigns indexes to messages, another endpoint should remove them. Otherwise workers further in delivery might wrongly interpret that data.

Or if messages were encrypted when entering a scope, they should be decrypted when leaving that scope.

## Data and metadata in scope context

Since Ockam Messaging combines multiple layers in delivery, what we consider "data"
and "metadata" depends on the current scope.

In order to modela every scope independently we can assume that all the information in the message which is external to the scope is "data portion" of the message.

Scope specific metadata only valid within the same scope.

For example in a scope of an indexed pipe, the message index is scope specific.

## Moving messages between scopes

As we model message delivery with wrapping and pipelining, we can model scopes as a stack:

- The bottom-level scope is the application data it needs to deliver.
- When message is forwarded through a different scope - it moves up the scope stack
- Moving up the scope stack, message is wrapped in the data portion of the upper level message format
- Moving down the scope stack, message is reconstructed from the data portion and upper level scope information is removed

**TODO: picture**

When forwarding through a scope, there is no requirement for messages leaving the scope to be exactly the same as they were entering the scope, but they must be in the same message format.


## Route scope

Some forwarding scopes, like pipes and channels need to preserve routes from the original scopes and may use some internal scopes for delivery.

**TODO: picture**

This makes routes a part of the scope metadata and they can be wrapped in the forwarding scope together with the rest of the message.

Alternative would be to extend the route with the forwarding scope routes, which is relatively simple, but to clean them up after exiting the forwarding scope is hard.


## Implementing scope data formats

In order to simplify moving messages between scopes across implementations, we can use a payload format proposed in [Payload Format](./Payload.md)
