defmodule Ockam.Services.Kafka.Provider do
  @moduledoc """
  Implementation for Ockam.Services.Provider
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

  @behaviour Ockam.Services.Provider

  alias Ockam.Kafka.Config, as: KafkaConfig
  alias Ockam.Stream.Index.Service, as: StreamIndexService
  alias Ockam.Stream.Workers.Service, as: StreamService

  @services [:stream_kafka, :stream_kafka_index]

  @impl true
  def services() do
    @services
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
    stream_prefix = KafkaConfig.stream_prefix(args)
    client_config = KafkaConfig.client_config(args)
    replication_factor = KafkaConfig.replication_factor(args)
    endpoints = KafkaConfig.endpoints(args)

    [
      replication_factor: replication_factor,
      endpoints: endpoints,
      client_config: client_config,
      topic_prefix: stream_prefix
    ]
  end
end
