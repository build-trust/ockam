# Telemetry

Ockam commands and nodes generate telemetry data in the form of Opentelemetry logs and spans.
This telemetry data is collected via [an Opentelemetry collector](https://opentelemetry.io/docs/collector). That
collector serves as a central point of collection and can forward this data to a variety of backends, such as:

- S3 for long-term storage.
- Observability systems like Honeycomb, DataClickHouse, etc... for analysis and visualization.

# Configuration

When the `OCKAM_OPENTELEMETRY_EXPORT` environment variable is set to `true`, there are various ways to send telemetry
data to the collector:

1. Via a secure channel to your project and then to the collector (the default).
1. Via a secure channel to your project's authority node and then to the collector.
1. Via a secure channel to an arbitrary Ockam node and then to the collector.
1. Directly via HTTP.

## Sending telemetry data directly via a secure channel to the project

This behaviour is controlled by the `OCKAM_TELEMETRY_EXPORT_VIA_PROJECT` environment variable (the default is `true`).

This mode is only active if a default project can be detected locally and is accessible via a secure channel.
In that case, the telemetry data is sent as Ockam messages to the project node and then forwarded to the collector.

### Services configuration

The project node must be started with the following configuration:

```elixir
  config :ockam_services,
    services:
      {:grpc_forwarder,
      [
        address: "grpc_forwarder",
        grpc_endpoint: "http://opentelemetry-collector:4317",
        authorization: [{Ockam.Worker.Authorization, :from_secure_channel}]
      ]}
```

The value of `grpc_endpoint` must be set to the endpoint of an accessible OpenTelemetry collector.

## Sending telemetry data directly via a secure channel to the authority node

This behaviour is controlled by the `OCKAM_TELEMETRY_EXPORT_VIA_PROJECT` environment variable (the default is `false`).

This mode is only active if a default project can be detected locally and is accessible via a secure channel.
Then, the telemetry data is sent as Ockam messages to the project's authority node and forwarded to the collector.

### Configuration

In order for the forwarding to work, the authority node be configured with the `OCKAM_OPENTELEMETRY_ENDPOINT` set to
the collector endpoint URL, for example `http://opentelemetry-collector:4317`.

## Sending telemetry data directly via a secure channel to an arbitrary Ockam node

This behaviour is controlled by the setting of two environment variables:

- `OCKAM_TELEMETRY_EXPORT_NODE_ROUTE` a route to the node to connect. For example:
  `/dnsaddr/localhost/tcp/4000/secure/api`. Note that the presence of `secure/api` in the address is going to trigger
  the creation of a secure channel.
- `OCKAM_TELEMETRY_EXPORT_NODE_IDENTIFIER` the identifier of the node that we are connecting to.
- `OCKAM_TELEMETRY_EXPORT_NODE_FORWARDER_SERVICE` the address of the `GrpcForwarder` service started on the remote node.
  The default is `grpc_forwarder` (see the project configuration above where that name is used to start the
  `GrpcForwarder` for example)

## Sending telemetry data directly via HTTP

In order to do this you need to set the following environment variables:

- `OCKAM_TELEMETRY_EXPORT=true`: this is the default value.
- `OCKAM_OPENTELEMETRY_ENDPOINT=http://opentelemetry-collector:4317`, assuming that your OpenTelemetry collector is
  running a gRPC endpoint on port 4317.

The `receivers` section of your collector configuration file should look like this:

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: "0.0.0.0:4317"
```

Notes:

- The `0.0.0.0` address, which enables the accessibility of the port 4317 on all network interfaces.
  This is in particular required if you deploy an Opentelemetry collector in a container.
- It is also possible to use a HTTPs address for the endpoint.

## Cutoff times

When a command is executed, log messages and spans cumulated in batches and those batches are then sent to the
collector. A few environment variables can be used to control the sending of these batches:

- `OCKAM_SPAN_EXPORT_TIMEOUT`: Timeout for trying to export spans. Default value: `5s`.
- `OCKAM_SPAN_EXPORT_QUEUE_SIZE`: Size of the queue used to store spans before they are sent. When the queue is full,
  spans are dropped. Default value: `32768`
- `OCKAM_FOREGROUND_SPAN_EXPORT_SCHEDULED_DELAY`: Maximum duration between the sending of two batches of spans. Default
  value: `1000s` (this value is high to avoid a deadlock in the tracing library).
- `OCKAM_FOREGROUND_SPAN_EXPORT_CUTOFF`: Cutoff time for sending a span batch, without waiting for a response. Default
  value: `3s`.

The same environment variables are available for logs instead of spans (replace `SPAN` by `LOG`) and for
a background node instead of a foreground node or command (replace `FOREGROUND` by `BACKGROUND`).

Default values are the same except for `OCKAM_BACKGROUND_SPAN/LOG_EXPORT_SCHEDULED_DELAY` which is set to `5s`.

# Debugging

The variable `OCKAM_OPENTELEMETRY_EXPORT_DEBUG` can be set to `true` to display info/debug and error messages related to
the setup of the telemetry sub-system.
