defmodule Ockam.HubNode.Cleanup do
  @moduledoc """
  Worker and topic cleanup helpers.
  """

  alias Ockam.HubNode.Cleanup.Kafka.TopicCleanup
  alias Ockam.HubNode.Cleanup.WorkerCleanup

  require Logger

  @doc """
  Cleanup idle forwarding workers, idle stream workers and optionally kafka topics

  Arguments:
  `idle_timeout` - integer: how long the workers and the topics should be idle to be removed, in ms
  `cleanup_kafka_topics` - boolean: whether to cleanup kafka topics
  """
  def cleanup_all(idle_timeout, cleanup_kafka_topics) do
    Logger.info("Start cleanup")
    Logger.info("Cleanup stream workers older than #{idle_timeout} ms")
    WorkerCleanup.cleanup_idle_workers(Ockam.Stream.Workers.Stream, idle_timeout)
    Logger.info("Cleanup stream index shards older than #{idle_timeout} ms")
    WorkerCleanup.cleanup_idle_workers(Ockam.Stream.Index.Shard, idle_timeout)
    Logger.info("Cleanup forwarders older than #{idle_timeout} ms")
    WorkerCleanup.cleanup_idle_workers(Ockam.Hub.Service.Forwarding.Forwarder, idle_timeout)

    case cleanup_kafka_topics do
      true ->
        Logger.info("Cleanup kafka topics older than #{idle_timeout} ms")
        TopicCleanup.cleanup_idle_topics(idle_timeout, [])

      false ->
        :ok
    end
  end
end
