## Ockam healthcheck

This application runs periodic healthcheck requests to ockam nodes by establishing
a secure channel and:
* sending a ping message to the configured worker for default targets
* sending an `Ockam.API.Request` message with a specified path, method and optionally a body for `Ockam.Services.API` endpoint targets, where `200` response is considered a healthy result

Healthcheck results are reported as prometheus metrics using `ockam_metrics` and
as logs using Logger.

## Configuration

Healthcheck targets and frequency of calling them are configured as: `:ockam_healthcheck, :targets` application environment or `HEALTHCHECK_TARGETS` environment variable
Format: application environment is a list of maps. For default (ping) targets the map is formatted as follows:
```elixir
%{
  name: ...,
  host: ...,
  port: ...,
  api_worker: ...,
  healthcheck_worker: ...,
  crontab: ...
}
```
For `Ockam.Services.API` endpoint targets:
```elixir
%{
  name: ...,
  host: ...,
  port: ...,
  path: ...,
  method: ...,
  body: ...,
  api_worker: ...,
  healthcheck_worker: ...,
  crontab: ...
}
```
An environment variable is a JSON string with the same format.
The `method` field has to be a string representation of one of the methods enumeratated in the `Ockam.API.Request.Header` schema - "get", "post", "put", "delete", or "patch".
The optional `body` binary for `Ockam.Services.API` endpoint targets in the JSON format has to be base64 encoded, such as with `Base.encode64/2`.

For each target, the application will connect via TCP `host:port` connection, establish an Ockam secure channel using `api_worker` listener and:
* for default targets send ping to `healthcheck_worker`
* for `Ockam.Services.API` endpoint targets send an `Ockam.API.Request` message with the specified `path`, `method` and optionally a `body` to `healthcheck_worker`

Frequency for each target is specified in the `crontab` field of that target as a string in the crontab format.

There is no way currently to control the targets or the frequency other than restarting the application.

Identity configuration for healthcheck calls:

Healthcheck connects to remote node as an identity and vault name configured using:
- source: `:ockam_healthcheck, :identity_source` (`HEALTHCHECK_IDENTITY_SOURCE`), can be either `:function` (default) or `:file`
- function: `:ockam_healthcheck, :identity_function`, defaults to `Ockam.Healthcheck.generate_identity`
- file: `:ockam_healthcheck, :identity_file` (`HEALTHCHECK_IDENTITY_FILE`), should contain binary identity data, only relevant if `:identity_source` is `:file`


Additional configuration for running standalone application:

- Identity implementation `IDENTITY_IMPLEMENTATION`
  Used to select identity implementation, defaults to `sidecar` requiring an identity sidecar to be running next to the node
- `OCKAM_SIDECAR_HOST` and `OCKAM_SIDECAR_PORT` - host and port to access identity sidecar
- `STORAGE` - directory where to cache healthcheck node identity identifier
- `PROMETHEUS_PORT` - port to use to report prometheus metrics
