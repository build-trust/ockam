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
  web_port: 4001

## Kafka config:

kafka_enabled = System.get_env("ENABLE_KAFKA", "false") == "true"

kafka_host = System.get_env("KAFKA_HOST", "localhost")
kafka_port = String.to_integer(System.get_env("KAFKA_PORT", "9092"))

kafka_sasl =
  case System.get_env("KAFKA_SASL") do
    nil -> nil
    string -> String.to_atom(string)
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
          ""
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
          ""
      end

    not_empty ->
      not_empty
  end

kafka_ssl = System.get_env("KAFKA_SSL") == "true"

kafka_client_config =
  case kafka_sasl do
    nil -> [ssl: kafka_ssl]
    sasl -> [sasl: {sasl, kafka_user, kafka_password}, ssl: kafka_ssl]
  end

kafka_replication_factor = String.to_integer(System.get_env("KAFKA_REPLICATION_FACTOR", "1"))

config :ockam_kafka,
  enabled: kafka_enabled,
  endpoints: [{kafka_host, kafka_port}],
  storage_options: [
    client_config: kafka_client_config,
    replication_factor: kafka_replication_factor
  ]
