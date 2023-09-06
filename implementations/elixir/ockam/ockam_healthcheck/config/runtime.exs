import Config

## Metrics config

# PROMETHEUS_PORT must be set for prometheus metrics to be enabled
config :ockam_metrics,
  include_node_metrics: false,
  prometheus_port: System.get_env("PROMETHEUS_PORT"),
  poller_measurements: [],
  metrics:
    Ockam.Healthcheck.Metrics.metrics() ++
      Ockam.Metrics.vm_metrics() ++ Ockam.Metrics.ockam_workers_metrics()

## Logger config

config :logger, level: :info

config :logger, :console,
  metadata: [:module, :line, :pid],
  format_string: "$dateT$time $metadata[$level] $message\n"

## Healthcheck targets config
targets_config = System.get_env("HEALTHCHECK_TARGETS", "[]")

targets =
  case Ockam.Healthcheck.Application.parse_config(targets_config) do
    {:ok, targets} ->
      targets

    {:error, reason} ->
      IO.puts(
        :stderr,
        "Invalid targets configuration #{inspect(targets_config)} : #{inspect(reason)}"
      )

      exit(:invalid_config)
  end

identity_source =
  case System.get_env("HEALTHCHECK_IDENTITY_SOURCE", "function") do
    "function" ->
      :function

    "file" ->
      :file
  end

identity_file = System.get_env("HEALTHCHECK_IDENTITY_FILE")
identity_signing_key_file = System.get_env("HEALTHCHECK_IDENTITY_SIGNING_KEY_FILE")

config :ockam_healthcheck,
  targets: targets,
  identity_source: identity_source,
  identity_file: identity_file,
  identity_signing_key_file: identity_signing_key_file,
  identity_function: &Ockam.Healthcheck.get_identity/0
