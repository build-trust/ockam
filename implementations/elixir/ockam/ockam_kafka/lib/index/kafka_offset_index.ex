defmodule Ockam.Stream.Index.KafkaOffset do
  @moduledoc """
    Kafka storage backend for ockam stream index service
    Using kafka offset storage
  """
  @behaviour Ockam.Stream.Index

  alias KafkaEx.Protocol.OffsetCommit
  alias KafkaEx.Protocol.OffsetFetch

  alias Ockam.Kafka

  require Logger

  @default_worker :kafka_index_worker

  @impl true
  def init(options) do
    ## TODO: fail if unable to create the worker
    Kafka.ensure_kafka_worker(options, @default_worker)
    {:ok, options}
  end

  @impl true
  def get_index(client_id, stream_name, partition, options) do
    worker_name = Kafka.worker_name(options, @default_worker)
    request = get_index_request(client_id, stream_name, partition, options)

    Logger.info("Get index request #{inspect(request)}")
    topic = Map.get(request, :topic)

    result =
      case KafkaEx.offset_fetch(worker_name, request) do
        [
          %OffsetFetch.Response{
            topic: ^topic,
            partitions: [partition_response]
          }
        ] ->
          case Map.get(partition_response, :error_code) do
            :no_error ->
              {:ok, Map.get(partition_response, :offset)}

            :unknown_topic_or_partition ->
              {:ok, :undefined}

            other ->
              {:error, {:get_error, other}}
          end

        other ->
          {:error, {:get_error, other}}
      end

    {result, options}
  end

  @impl true
  def save_index(client_id, stream_name, partition, index, options) do
    worker_name = Kafka.worker_name(options, @default_worker)
    request = save_index_request(client_id, stream_name, partition, index, options)
    topic = Map.get(request, :topic)
    partition = Map.get(request, :partition)

    result =
      case KafkaEx.offset_commit(worker_name, request) do
        [%OffsetCommit.Response{partitions: [^partition], topic: ^topic}] ->
          :ok

        other ->
          {:error, {:save_error, other}}
      end

    {result, options}
  end

  def reset(client_id, stream_name, partition, options) do
    save_index(client_id, stream_name, partition, 0, options)
  end

  def get_index_request(client_id, stream_name, partition, options) do
    ## TODO: topic/partition
    %OffsetFetch.Request{
      consumer_group: client_id,
      topic: Kafka.topic(stream_name, options),
      partition: Kafka.partition(stream_name, partition, options)
    }
  end

  def save_index_request(client_id, stream_name, partition, index, options) do
    %OffsetCommit.Request{
      consumer_group: client_id,
      topic: Kafka.topic(stream_name, options),
      partition: Kafka.partition(stream_name, partition, options),
      offset: index
    }
  end
end
