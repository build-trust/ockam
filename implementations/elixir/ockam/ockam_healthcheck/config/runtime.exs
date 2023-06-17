import Config

## Ockam identity config

identity_module =
  case System.get_env("IDENTITY_IMPLEMENTATION", "") do
    "sidecar" ->
      Ockam.Identity.Sidecar

    "stub" ->
      Ockam.Identity.Stub

    "" ->
      case Mix.env() do
        :test ->
          Ockam.Identity.Stub

        _other ->
          Ockam.Identity.Sidecar
      end

    other ->
      IO.puts(:stderr, "Unknown identity implementation: #{inspect(other)}")
      exit(:invalid_config)
  end

config :ockam, identity_module: identity_module

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

## Services config

sidecar_host = System.get_env("OCKAM_SIDECAR_HOST", "localhost")
sidecar_port = String.to_integer(System.get_env("OCKAM_SIDECAR_PORT", "4100"))

identity_sidecar_services =
  case identity_module do
    Ockam.Identity.Sidecar ->
      [
        identity_sidecar: [
          authorization: [:is_local],
          sidecar_host: sidecar_host,
          sidecar_port: sidecar_port
        ]
      ]

    _ ->
      []
  end

config :ockam_services,
  service_providers: [
    # sidecar services
    Ockam.Services.Provider.Sidecar
  ],
  ## Start services by default
  services: identity_sidecar_services

## Healthcheck targets config
targets_config = System.get_env("HEALTHCHECK_TARGETS", "[]")
api_endpoint_targets_config = System.get_env("HEALTHCHECK_API_ENDPOINT_TARGETS", "[]")

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

api_endpoint_targets =
  case Ockam.Healthcheck.Application.parse_config(api_endpoint_targets_config) do
    {:ok, api_endpoint_tagrets} ->
      api_endpoint_tagrets

    {:error, reason} ->
      IO.puts(
        :stderr,
        "Invalid API endpoint targets configuration #{inspect(api_endpoint_targets_config)} : #{inspect(reason)}"
      )

      exit(:invalid_config)
  end

crontab = System.get_env("HEALTHCHECK_CRONTAB")
api_endpoint_crontab = System.get_env("HEALTHCHECK_API_ENDPOINT_CRONTAB")

identity_source =
  case System.get_env("HEALTHCHECK_IDENTITY_SOURCE", "function") do
    "function" ->
      :function

    "file" ->
      :file
  end

identity_file = System.get_env("HEALTHCHECK_IDENTITY_FILE")

config :ockam_healthcheck,
  crontab: crontab,
  api_endpoint_crontab: api_endpoint_crontab,
  targets: targets,
  api_endpoint_targets: api_endpoint_targets,
  identity_source: identity_source,
  identity_file: identity_file,
  identity_function: &Ockam.Healthcheck.generate_identity/0
