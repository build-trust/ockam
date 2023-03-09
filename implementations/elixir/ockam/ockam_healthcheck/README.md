## Ockam healthcheck

This application runs periodic healthcheck requests to ockam nodes by establishing
a secure channel and sending a ping message to the configured worker.

Healthcheck results are reported as prometheus metrics using `ockam_metrics` and
as logs using Logger.

## Configuration

There are two main configurations:

- frequency
  Configured as: `:ockam_healthcheck, :crontab` application environment or `HEALTHCHECK_CRONTAB` environment vatiable
  Format: string in crontab format
- healthcheck targets
  Configured as: `:ockam_healthcheck, :targets` application environment or `HEALTHCHECK_TARGETS` environment vatiable
  Format: application environment is a list of maps `[%{name: ..., host: ..., port: ..., api_worker: ..., healthcheck_worker: ...}, ...]`, environment variable is a JSON string with the same format

For each target, the application will connect via tcp `host:port` connection, establish secure channel
using `api_worker` listener and send ping to `healthcheck_worker`.

Each cycle of crontab frequency the targets configuration will be read from the `:ockam_healthcheck, :targets`, which allows Elixir node to control targets.

There is no way currently to control the frequency other than restarting the application.

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
