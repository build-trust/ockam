defmodule Ockam.HubNode.Cleanup.Kafka.TopicCleanup do
  @moduledoc """
  Helper module to cleanup idle kafka topics
  """

  require Record

  Record.defrecord(
    :kafka_message,
    Record.extract(:kafka_message, from_lib: "kafka_protocol/include/kpro_public.hrl")
  )

  @fetch_options %{
    max_wait_time: 1,
    min_bytes: 1,
    max_bytes: 1_048_576
  }

  def find_idle_topics(idle_time, options) do
    now = System.os_time(:millisecond)
    expired_time = now - idle_time

    Ockam.Kafka.get_topics(options)
    |> Enum.filter(fn topic ->
      idle_topic?(topic, expired_time, options)
    end)
    |> Enum.map(fn {topic_name, _} -> topic_name end)
  end

  def cleanup_idle_topics(idle_time, options) do
    idle_time
    |> find_idle_topics(options)
    |> cleanup_topics(options)
  end

  def cleanup_topics(topics, options) do
    Ockam.Kafka.delete_topics(topics, options)
  end

  def idle_topic?({topic_name, partitions}, expired_time, options) do
    Enum.all?(:lists.seq(0, partitions - 1), fn partition ->
      idle_partition?(topic_name, partition, expired_time, options)
    end)
  end

  def idle_partition?(topic_name, partition, expired_time, options) do
    case Ockam.Kafka.resolve_offset(topic_name, partition, :latest, options) do
      {:ok, offset} when offset > 0 ->
        case Ockam.Kafka.fetch(topic_name, partition, offset - 1, @fetch_options, options) do
          {:ok, messages} ->
            idle_messages?(messages, expired_time)

          _other ->
            false
        end

      _other ->
        false
    end
  end

  def idle_messages?(messages, expired_time) do
    Enum.all?(messages, fn message ->
      case message do
        kafka_message(ts: ts) ->
          ts < expired_time

        _other ->
          false
      end
    end)
  end
end
