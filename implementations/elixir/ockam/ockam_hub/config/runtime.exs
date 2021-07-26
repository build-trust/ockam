import Config

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

config :ockam_hub, :influxdb,
  host: System.get_env("INFLUXDB_HOST"),
  port: System.get_env("INFLUXDB_PORT"),
  bucket: System.get_env("INFLUXDB_BUCKET"),
  org: System.get_env("INFLUXDB_ORG"),
  token: influx_token

ui_auth_message =
  with true <- File.exists?("/mnt/secrets/auth/message"),
       {:ok, contents} <- File.read("/mnt/secrets/auth/message"),
       client_secret <- String.trim(contents) do
    client_secret
  else
    false ->
      System.get_env("AUTH_MESSAGE") || "devsecret"

    {:error, :enoent} ->
      System.get_env("AUTH_MESSAGE") || "devsecret"
  end

ui_auth_host =
  with true <- File.exists?("/mnt/secrets/auth/host"),
       {:ok, contents} <- File.read("/mnt/secrets/auth/host"),
       client_secret <- String.trim(contents) do
    client_secret
  else
    false ->
      System.get_env("AUTH_HOST") || "http://localhost:4002"

    {:error, :enoent} ->
      System.get_env("AUTH_HOST") || "http://localhost:4002"
  end

node_fqdn =
  case System.get_env("NODE_FQDN") do
    fqdn when is_binary(fqdn) and fqdn != "" ->
      fqdn

    _ ->
      case config_env() do
        :dev -> "localhost"
        :test -> "localhost"
        _ -> "1.node.ockam.network"
      end
  end

node_ip =
  case config_env() do
    :prod ->
      {:ok, {:hostent, _, _, :inet, 4, [node_ip]}} = :inet.gethostbyname(to_charlist(node_fqdn))

      node_ip

    :test ->
      {127, 0, 0, 1}

    :dev ->
      {127, 0, 0, 1}
  end

config :ockam_hub,
  auth_message: ui_auth_message,
  auth_host: ui_auth_host,
  node_ip: node_ip,
  node_fqdn: node_fqdn,
  tcp_transport_port: 4000,
  udp_transport_port: 7000,
  web_port: 4001

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
    Ockam.Kafka.Hub.Service.Provider
  ],
  services_config_source: services_config_source,
  # JSON version of the services definition
  services_json: services_json,
  services_file: services_file,
  services_list: services_list,
  ## Start echo and forwarding services by default
  services: [
    :echo,
    :forwarding,
    :tracing
  ]
