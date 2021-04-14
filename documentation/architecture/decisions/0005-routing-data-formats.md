# 5. Routing protocol data formats

Date: 2021-03-25

## Status

Proposed

## Context

We need a standard format for messages to be exchanged by the routing protocol.
This format would be used by routers on different implementations.

## Decision

We use the following formats:

For a message:

```
{
  onward_route: Route,
  return_route: Route,
  payload: Any
}
```

Where

`Route` - an ordered list of addresses.


For an address:

```
{
  type: Integer,
  data: Any
}
```

## Consequences

Applications can use the same format to implement handling code.

Transports between different implementations would have to agree on a certain
format to exchange payloads and addresses.

Different nodes have to agree on address types to exchange messages with such types
in the routes


