defmodule Ockam.Kafka do
  @moduledoc """
  Helper functions for ockam kafka backends
  """

  alias Ockam.Kafka.Config

  require Logger

  def client_id(options, default) do
    Keyword.get(options, :client_id, default)
  end

  def endpoints(options) do
    Keyword.get(options, :endpoints, Application.get_env(:ockam_kafka, :endpoints))
  end

  def request_configs(_options) do
    %{timeout: 300_000}
  end

  def topic(stream_name, options) do
    prefix = Keyword.get(options, :topic_prefix, "")
    prefix <> stream_name
  end

  def partition(_stream_name, partition, _options) do
    partition
  end

  def consumer_id(client_id, _options) do
    client_id
  end

  def client_config(options) do
    producer_config = [
      {:auto_start_producers, true},
      {:default_producer_config, []}
    ]

    timeout_config = [
      {:timeout, 20_000},
      {:request_timeout, 20_000},
      {:connect_timeout, 20_000},
      {:get_metadata_timeout_seconds, 20}
    ]

    default_config = Keyword.merge(producer_config, timeout_config)

    Keyword.merge(default_config, Keyword.get(options, :client_config, []))
  end

  def generate_client_id(stream_name, partition, options) do
    topic = topic(stream_name, options)
    client_name = "#{topic}_#{partition}_client"
    String.to_atom(client_name)
  end

  def create_client(options, default_client_id) do
    endpoints = endpoints(options)
    client_id = client_id(options, default_client_id)
    client_config = client_config(options)

    Logger.debug("Create client config: #{inspect(client_config)}")
    ## TODO: use supervised client
    ## TODO: associate client id with the current service / use unique ids or clients without ids
    case :brod.start_link_client(endpoints, client_id, client_config) do
      {:ok, client} -> {:ok, client}
      {:error, {:already_started, client}} -> {:ok, client}
      {:error, err} -> {:error, err}
    end
  end

  def create_topic(topic, partitions, options) do
    endpoints = endpoints(options)
    topic_configs = topic_configs(topic, partitions, options)
    request_configs = request_configs(options)
    client_config = client_config(options)

    Logger.debug(
      "Create topics: #{inspect([endpoints, topic_configs, request_configs, client_config])}"
    )

    case :brod.create_topics(endpoints, topic_configs, request_configs, client_config) do
      :ok -> :ok
      ## TODO: optional failure
      {:error, :topic_already_exists} -> :ok
      {:error, err} -> {:error, err}
    end
  end

  def get_topics(options) do
    brod_options = brod_options(options)

    case :brod.get_metadata(brod_options.endpoints, :all, brod_options.client_config) do
      {:ok, %{topic_metadata: topics}} ->
        Enum.map(topics, fn topic ->
          {Map.get(topic, :topic), Enum.count(Map.get(topic, :partition_metadata))}
        end)

      other ->
        other
    end
  end

  def resolve_offset(topic, partition, time, options) do
    brod_options = brod_options(options)

    :brod.resolve_offset(
      brod_options.endpoints,
      topic,
      partition,
      time,
      brod_options.client_config
    )
  end

  def fetch(topic, partition, offset, fetch_options, options) do
    brod_options = brod_options(options)

    :brod.fetch(
      brod_options.endpoints,
      topic,
      partition,
      offset,
      fetch_options.max_wait_time,
      fetch_options.min_bytes,
      fetch_options.max_bytes,
      brod_options.client_config
    )
  end

  def delete_topics(topics, options) do
    brod_options = brod_options(options)

    :brod.delete_topics(brod_options.endpoints, topics, 100_000, brod_options.client_config)
  end

  def brod_options(options) do
    endpoints = Config.endpoints(options)
    ## TODO: this is not pretty
    client_config = client_config(client_config: Config.client_config(options))

    %{
      endpoints: endpoints,
      client_config: client_config
    }
  end

  ## TODO: pass more options here
  def topic_configs(topic, partitions, options) do
    [
      %{
        replication_factor: Keyword.get(options, :replication_factor, 1),
        replica_assignment: [],
        config_entries: [],
        num_partitions: partitions,
        topic: topic
      }
    ]
  end
end
