defmodule Ockam.Hub.StreamSpawner do
  @moduledoc """
  Management module to create stream services for users
  """
  alias Ockam.Stream.Index.Worker, as: StreamIndexService
  alias Ockam.Stream.Workers.Service, as: StreamService

  require Logger

  def create_kafka_service(prefix) do
    if Application.get_env(:ockam_kafka, :enabled, false) do
      index_name = "stream_#{prefix}_index"
      service_name = "stream_#{prefix}_service"
      {:ok, stream_service} = create_stream_service(prefix, service_name, :kafka)
      {:ok, index_service} = create_stream_index_service(prefix, index_name, :kafka)

      %{
        stream_service: stream_service,
        index_service: index_service
      }
    else
      Logger.warn("Cannot create stream services: kafka disabled")
      %{}
    end
  end

  def create_stream_index_service(prefix, name, type) do
    storage_mod = index_storage_mod(type)
    storage_options = index_storage_options(type, prefix)

    ensure_service(StreamIndexService, name,
      storage_mod: storage_mod,
      storage_options: storage_options
    )
  end

  def create_stream_service(prefix, name, type) do
    storage_mod = stream_storage_mod(type)
    storage_options = stream_storage_options(type, prefix)

    ensure_service(StreamService, name,
      stream_options: [
        storage_mod: storage_mod,
        storage_options: storage_options
      ]
    )
  end

  def ensure_service(service, name, options) do
    case Ockam.Node.whereis(name) do
      nil ->
        service.create([{:address, name} | options])

      pid when is_pid(pid) ->
        {:ok, name}
    end
  end

  def stream_storage_mod(:internal), do: Ockam.Stream.Storage.Internal
  def stream_storage_mod(:kafka), do: Ockam.Stream.Storage.Kafka

  def index_storage_mod(:internal), do: Ockam.Stream.Index.Storage.Internal
  def index_storage_mod(:kafka), do: Ockam.Stream.Index.KafkaOffset

  def index_storage_options(:kafka, prefix) do
    kafka_storage_options = Application.get_env(:ockam_kafka, :storage_options, [])
    Keyword.put(kafka_storage_options, :topic_prefix, "#{prefix}_")
  end

  def index_storage_options(:internal, _prefix) do
    []
  end

  def stream_storage_options(:kafka, prefix) do
    kafka_storage_options = Application.get_env(:ockam_kafka, :storage_options, [])
    Keyword.put(kafka_storage_options, :topic_prefix, "#{prefix}_")
  end

  def stream_storage_options(:internal, _prefix) do
    []
  end
end
