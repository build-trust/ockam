defmodule Ockam.Kafka do
  @moduledoc """
  Helper functions for ockam kafka backends
  """

  require Logger

  def ensure_kafka_worker(options, default_worker) do
    worker_name = worker_name(options, default_worker)
    Logger.info("Create worker #{inspect(options)} #{inspect(worker_name)}")
    ## TODO: pass worker config
    case KafkaEx.create_worker(worker_name) do
      {:ok, pid} -> {:ok, pid}
      {:error, {:already_starter, pid}} -> {:ok, pid}
      other -> other
    end
  end

  def worker_name(options, default) do
    Keyword.get(options, :worker_name, default)
  end

  def topic(stream_name, _options) do
    stream_name
  end

  def partition(_stream_name, partition, _options) do
    partition
  end
end
