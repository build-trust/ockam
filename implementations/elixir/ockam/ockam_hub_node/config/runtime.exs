import Config

# Ockam Hub application config

## Token manager config

token_manager_cloud_service_module =
  case System.get_env("TOKEN_MANAGER_CLOUD_SERVICE", "INFLUXDB") do
    "INFLUXDB" -> Ockam.TokenLeaseManager.CloudService.Influxdb
  end

token_manager_storage_service_module =
  case System.get_env("TOKEN_MANAGER_STORAGE_SERVICE", "POSTGRES") do
    "POSTGRES" -> Ockam.TokenLeaseManager.StorageService.Postgres
  end

config :ockam_hub, :token_manager,
  cloud_service_module: token_manager_cloud_service_module,
  storage_service_module: token_manager_storage_service_module,
  cloud_service_options: [
    endpoint: System.get_env("HUB_NODE_INFLUXDB_ENDPOINT"),
    token: System.get_env("HUB_NODE_INFLUXDB_TOKEN"),
    org: System.get_env("HUB_NODE_INFLUXDB_ORG")
  ],
  storage_service_options: [
    hostname: System.get_env("POSTGRES_HOST"),
    port: String.to_integer(System.get_env("POSTGRES_PORT", "5432")),
    username: System.get_env("POSTGRES_USERNAME"),
    password: System.get_env("POSTGRES_PASSWORD"),
    database: System.get_env("POSTGRES_DATABASE")
  ]

## Transports config

config :ockam_hub,
  tcp_transport_port: 4000,
  udp_transport_port: 7000

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

## Services config

services_json = System.get_env("SERVICES_JSON")

services_list = System.get_env("SERVICES_LIST")

services_file = System.get_env("SERVICES_FILE")

services_config_source = System.get_env("SERVICES_CONFIG_SOURCE")

config :ockam_hub,
  service_providers: [
    # default services
    Ockam.Hub.Service.Provider.Routing,
    # stream services
    Ockam.Hub.Service.Provider.Stream,
    # kafka services
    Ockam.Kafka.Hub.Service.Provider,
    # token lease services
    Ockam.TokenLeaseManager.Hub.Service.Provider,
    # secure channel services
    Ockam.Hub.Service.Provider.SecureChannel,
    # discovery service
    Ockam.Hub.Service.Provider.Discovery
  ],
  services_config_source: services_config_source,
  # JSON version of the services definition
  services_json: services_json,
  services_file: services_file,
  services_list: services_list,
  ## Start echo and forwarding services by default
  services: [
    :discovery,
    :echo,
    :forwarding,
    :static_forwarding,
    :pub_sub,
    :stream,
    :stream_index,
    :secure_channel,
    :tracing
  ]

# Ockam Hub Node application config

## InfluxDB metrics config

influx_token =
  with true <- File.exists?("/mnt/secrets/influx/token"),
       {:ok, contents} <- File.read("/mnt/secrets/influx/token"),
       client_secret <- String.trim(contents) do
    client_secret
  else
    false ->
      System.get_env("INFLUXDB_TOKEN")

    {:error, :enoent} ->
      System.get_env("INFLUXDB_TOKEN")
  end

config :telemetry_influxdb,
  host: System.get_env("INFLUXDB_HOST"),
  port: System.get_env("INFLUXDB_PORT"),
  bucket: System.get_env("INFLUXDB_BUCKET"),
  org: System.get_env("INFLUXDB_ORG"),
  token: influx_token

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

config :ockam_hub_node,
  cleanup: [
    crontab: cleanup_crontab,
    idle_timeout: cleanup_idle_timeout,
    cleanup_kafka_topics: cleanup_kafka_topics
  ]

## Logger config

config :logger, level: :info

config :logger, :console,
  metadata: [:module, :line, :pid],
  format_string: "$dateT$time $metadata[$level] $message\n",
  format: {Ockam.HubNode.LogFormatter, :format}
