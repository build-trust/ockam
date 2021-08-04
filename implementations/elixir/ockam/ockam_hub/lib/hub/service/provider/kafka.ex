## TODO: this needs to be moved to ockam_kafka after we create ockam_hub_core
defmodule Ockam.Kafka.Hub.Service.Provider do
  @moduledoc """
  Implementation for Ockam.Hub.Service.Provider
  providing kafka stream services, :stream_kafka and :stream_kafka_index

  Services arguments:
  stream_kafka:
    address_prefix: optional<string>, worker address prefix
    stream_prefix: optional<string>, kafka topic prefix
    endpoints: optional<string | [string] | [{string, integer}]>, kafka bootstrap endpoints, defaults to "localhost:9092"
    user: optional<string>, kafka SASL username
    password: optional<string>, kafka SASL password, defaults to "" if only user is set
    sasl: optional<atom|string>, kafka sasl mode, defaults to "plain"
    ssl: optional<boolean>, if kafka server using ssl, defaults to false
    replication_factor: optional<integer> replication factor for topics, defaults to 1

  stream_kafka_index:
    address_prefix: optional<string>, worker address prefix
    stream_prefix: optional<string>, kafka topic prefix
    endpoints: optional<string | [string] | [{string, integer}]>, kafka bootstrap endpoints, defaults to "localhost:9092"
    user: optional<string>, kafka SASL username
    password: optional<string>, kafka SASL password, defaults to "" if only user is set
    sasl: optional<atom|string>, kafka sasl mode, defaults to "plain"
    ssl: optional<boolean> if kafka server using ssl, defaults to false
  """

  @behaviour Ockam.Hub.Service.Provider

  alias Ockam.Stream.Index.Service, as: StreamIndexService
  alias Ockam.Stream.Workers.Service, as: StreamService

  @services [:stream_kafka, :stream_kafka_index]

  @impl true
  def services() do
    @services
  end

  @impl true
  def start_service(service_name, args) do
    options = service_options(service_name, args)
    mod = service_mod(service_name)
    mod.create(options)
  end

  @impl true
  def child_spec(service_name, args) do
    options = service_options(service_name, args)
    mod = service_mod(service_name)
    {mod, options}
  end

  def service_mod(:stream_kafka) do
    StreamService
  end

  def service_mod(:stream_kafka_index) do
    StreamIndexService
  end

  def service_options(:stream_kafka, args) do
    address = make_address(args, "stream_kafka")

    stream_options = [
      storage_mod: Ockam.Stream.Storage.Kafka,
      storage_options: storage_options(args)
    ]

    [address: address, stream_options: stream_options]
  end

  def service_options(:stream_kafka_index, args) do
    address = make_address(args, "stream_kafka_index")

    [
      address: address,
      storage_mod: Ockam.Stream.Index.KafkaOffset,
      storage_options: storage_options(args)
    ]
  end

  def make_address(args, default_address) do
    address_prefix = Keyword.get(args, :address_prefix, "")
    base_address = Keyword.get(args, :address, default_address)
    prefix_address(base_address, address_prefix)
  end

  def prefix_address(base_address, "") do
    base_address
  end

  def prefix_address(base_address, prefix) do
    prefix <> "_" <> base_address
  end

  def storage_options(args) do
    stream_prefix =
      Keyword.get(args, :stream_prefix, Application.get_env(:ockam_kafka, :stream_prefix, ""))

    prefix =
      case stream_prefix do
        "" -> ""
        string -> "#{string}_"
      end

    sasl_options = sasl_options(args)
    ssl = Keyword.get(args, :ssl, Application.get_env(:ockam_kafka, :ssl))

    replication_factor =
      Keyword.get(
        args,
        :replication_factor,
        Application.get_env(:ockam_kafka, :replication_factor)
      )

    endpoints = endpoints(args)

    [
      replication_factor: replication_factor,
      endpoints: endpoints,
      client_config: [ssl: ssl] ++ sasl_options,
      topic_prefix: prefix
    ]
  end

  def sasl_options(args) do
    sasl =
      args |> Keyword.get(:sasl, Application.get_env(:ockam_kafka, :sasl)) |> String.to_atom()

    user = Keyword.get(args, :user, Application.get_env(:ockam_kafka, :user))
    password = Keyword.get(args, :password, Application.get_env(:ockam_kafka, :password))

    case user do
      nil ->
        []

      _defined ->
        [sasl: {sasl, user, password}]
    end
  end

  def endpoints(args) do
    args
    |> Keyword.get(:endpoints, Application.get_env(:ockam_kafka, :endpoints))
    |> parse_endpoints()
  end

  def parse_endpoints(endpoints) when is_list(endpoints) do
    Enum.map(endpoints, fn string when is_binary(string) ->
      with [host, port_str] <- String.split(string, ":"),
           port_int <- String.to_integer(port_str) do
        {host, port_int}
      else
        err ->
          raise("Unable to parse kafka endpoints: #{inspect(endpoints)}: #{inspect(err)}")
      end
    end)
  end

  def parse_endpoints(endpoints) do
    parse_endpoints(String.split(endpoints, ","))
  end
end
