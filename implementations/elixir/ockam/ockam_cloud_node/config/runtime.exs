import Config

# Ockam Cloud Node application config

## Transports config

tcp_port = String.to_integer(System.get_env("TCP_PORT", "4000"))
udp_port = String.to_integer(System.get_env("UDP_PORT", "7000"))

config :ockam_services,
  tcp_transport: [listen: [port: tcp_port]]

## Kafka default config

kafka_endpoints = System.get_env("KAFKA_ENDPOINTS", "localhost:9092")

kafka_sasl =
  case System.get_env("KAFKA_SASL") do
    empty when empty == nil or empty == "" ->
      "plain"

    not_empty ->
      not_empty
  end

kafka_user = System.get_env("KAFKA_USER")
kafka_password = System.get_env("KAFKA_PASSWORD")

kafka_user =
  case kafka_user do
    empty when empty == nil or empty == "" ->
      with true <- File.exists?("/mnt/secrets/kafka/user"),
           {:ok, contents} <- File.read("/mnt/secrets/kafka/user"),
           data <- String.trim(contents) do
        data
      else
        _ ->
          IO.puts(:stderr, "Kafka user is not configured")
          nil
      end

    not_empty ->
      not_empty
  end

kafka_password =
  case kafka_password do
    empty when empty == nil or empty == "" ->
      with true <- File.exists?("/mnt/secrets/kafka/password"),
           {:ok, contents} <- File.read("/mnt/secrets/kafka/password"),
           data <- String.trim(contents) do
        data
      else
        _ ->
          IO.puts(:stderr, "Kafka password is not configured")
          nil
      end

    not_empty ->
      not_empty
  end

kafka_ssl = System.get_env("KAFKA_SSL") == "true"

kafka_replication_factor = String.to_integer(System.get_env("KAFKA_REPLICATION_FACTOR", "1"))

kafka_stream_prefix = System.get_env("KAFKA_STREAM_PREFIX") || ""

config :ockam_kafka,
  endpoints: kafka_endpoints,
  replication_factor: kafka_replication_factor,
  ssl: kafka_ssl,
  sasl: kafka_sasl,
  user: kafka_user,
  password: kafka_password,
  stream_prefix: kafka_stream_prefix

## Identity secure channel config

identity_module =
  case System.get_env("IDENTITY_IMPLEMENTATION", "stub") do
    "sidecar" ->
      Ockam.Identity.Sidecar

    "stub" ->
      Ockam.Identity.Stub

    other ->
      IO.puts(:stderr, "Unknown identity implementation: #{inspect(other)}")
      exit(:invalid_config)
  end

config :ockam, identity_module: identity_module

## Services config

services_list = System.get_env("SERVICES_LIST", "")

services =
  String.split(services_list, ",", trim: true)
  |> Enum.map(fn name -> String.trim(name) |> String.to_atom() end)

config :ockam_services,
  service_providers: [
    # default services
    Ockam.Services.Provider.Routing,
    # stream services
    Ockam.Services.Provider.Stream,
    # kafka services
    Ockam.Services.Kafka.Provider,
    # token lease services
    Ockam.Services.TokenLeaseManager.Provider,
    # secure channel services
    Ockam.Services.Provider.SecureChannel,
    # discovery service
    Ockam.Services.Provider.Discovery,
    # proxies for remote services
    Ockam.Services.Provider.Proxy,
    # proxies to services in other nodes
    Ockam.Services.Provider.Sidecar
  ],
  services: services

# Ockam Cloud Node application config

## Auto cleanup config

cleanup_crontab = System.get_env("CLEANUP_CRONTAB")

cleanup_idle_timeout =
  case System.get_env("CLEANUP_IDLE_TIMEOUT") do
    nil ->
      nil

    "" ->
      nil

    val ->
      case Integer.parse(val) do
        {int, ""} ->
          int

        _ ->
          IO.puts("Invalid CLEANUP_IDLE_TIMEOUT: #{val}. Ignoring")
          nil
      end
  end

cleanup_kafka_topics = System.get_env("CLEANUP_KAFKA_TOPICS", "false") == "true"

config :ockam_cloud_node,
  cleanup: [
    crontab: cleanup_crontab,
    idle_timeout: cleanup_idle_timeout,
    cleanup_kafka_topics: cleanup_kafka_topics
  ]

## ABAC configuration

abac_policy_storage =
  case System.get_env("ABAC_POLICY_STORAGE", "memory") do
    "memory" ->
      Ockam.ABAC.PolicyStorage.ETS

    "file" ->
      Ockam.ABAC.PolicyStorage.DETS

    "file:" <> filename ->
      {Ockam.ABAC.PolicyStorage.DETS, [filename: filename]}
  end

config :ockam_abac, policy_storage: abac_policy_storage

# must be set for prometheus metrics to be enabled
config :ockam_metrics,
  prometheus_port: System.get_env("PROMETHEUS_PORT"),
  poller_measurements: Ockam.Services.Metrics.TelemetryPoller.measurements(),
  metrics_fun: {Ockam.Services.Metrics, :metrics, []}

## Logger config

config :logger, level: :info

config :logger, :console,
  metadata: [:module, :line, :pid],
  format_string: "$dateT$time $metadata[$level] $message\n",
  format: {Ockam.CloudNode.LogFormatter, :format}
