defmodule Ockam.Stream.Storage.Kafka do
  @moduledoc """
    Kafka storage backend for ockam stream service
  """
  @behaviour Ockam.Stream.Storage

  alias KafkaEx.Protocol.CreateTopics
  alias KafkaEx.Protocol.Fetch
  alias KafkaEx.Protocol.Produce

  alias Ockam.Kafka

  require Logger

  @type options() :: Keyword.t()
  @type message() :: %{index: integer(), data: binary()}

  @default_worker :ockam

  ## Storage API
  @impl true
  @spec init_stream(String.t(), integer(), options()) :: {:ok, options()} | {:error, any()}
  ## TODO: call this from stream service
  def init_stream(stream_name, partitions, options) do
    Logger.info("Init stream #{inspect(stream_name)} #{partitions}")
    request = create_topics_request(stream_name, partitions, options)
    worker_name = Kafka.worker_name(options, @default_worker)

    no_error = topic_error_none(request)
    topic_already_exists = topic_error_exists(request)
    Logger.info("Kafka storage init #{inspect(request)}")

    ## TODO: fail if unable to create the worker
    Kafka.ensure_kafka_worker(options, @default_worker)

    case KafkaEx.create_topics([request], worker_name: worker_name) do
      %CreateTopics.Response{
        topic_errors: [^no_error]
      } ->
        {:ok, options}

      %CreateTopics.Response{
        topic_errors: [^topic_already_exists]
      } ->
        Logger.info("Using existing topic")
        {:ok, options}

      other ->
        ## TODO: parse the error
        Logger.error("Create topic error #{inspect(other)}")
        {:error, other}
    end
  end

  @impl true
  @spec init_partition(String.t(), integer(), any(), options()) ::
          {:ok, options()} | {:error, any()}
  def init_partition(_stream_name, _partition, _state, options) do
    {:ok, options}
  end

  @impl true
  @spec save(String.t(), integer(), binary(), options()) ::
          {{:ok, integer()} | {:error, any()}, options()}
  def save(stream_name, partition, message, options) do
    Logger.info("Save #{inspect(stream_name)} #{partition}")
    request = produce_request(stream_name, partition, message, options)
    worker_name = Kafka.worker_name(options, @default_worker)

    result =
      case KafkaEx.produce(request, worker_name: worker_name) do
        :ok ->
          {:ok, 0}

        {:ok, index} ->
          {:ok, index}

        {:error, err} ->
          {:error, err}

        other ->
          Logger.error("Unexpected result from produce: #{inspect(other)}")
          {:error, {:save_response, other}}
      end

    {result, options}
  end

  @impl true
  @spec fetch(String.t(), integer(), integer(), integer(), options()) ::
          {{:ok, [message()]} | {:error, any()}, options()}
  def fetch(stream_name, partition, index, limit, options) do
    Logger.info("Fetch #{stream_name} #{partition}, #{index}")
    topic = Kafka.topic(stream_name, options)
    partition = Kafka.partition(stream_name, partition, options)
    fetch_options = fetch_options(stream_name, index, limit, options)

    {fetch_messages(limit, topic, partition, fetch_options), options}
  end

  defp fetch_messages(limit, topic, partition, fetch_options, previous \\ []) do
    prev_count = Enum.count(previous)

    case KafkaEx.fetch(topic, partition, fetch_options) do
      :topic_not_found ->
        {:error, :topic_not_found}

      [%Fetch.Response{topic: topic} = fetch_response] ->
        messages = get_response_messages(fetch_response)

        case Enum.count(messages) do
          0 ->
            {:ok, previous}

          num when num + prev_count >= limit ->
            {:ok, previous ++ Enum.take(messages, limit)}

          _other ->
            last_index = last_index(messages)
            fetch_options = fetch_options(fetch_options, last_index + 1)
            fetch_messages(limit, topic, partition, fetch_options, previous ++ messages)
        end

      other ->
        Logger.error("Unexpected fetch response #{inspect(other)}")
        {:error, {:fetch_response, other}}
    end
  end

  def get_response_messages(%Fetch.Response{partitions: partitions}) do
    partitions
    |> Enum.flat_map(fn partition ->
      partition
      |> Map.get(:message_set, [])
      |> Enum.map(fn %{offset: offset, value: value} -> %{index: offset, data: value} end)
    end)
    |> Enum.sort_by(fn %{index: index} -> index end)
  end

  def last_index(messages) do
    messages
    |> Enum.map(fn %{index: index} -> index end)
    |> Enum.max()
  end

  defp create_topics_request(stream_name, partitions, options) do
    topic = Kafka.topic(stream_name, options)
    %CreateTopics.TopicRequest{topic: topic, num_partitions: partitions}
  end

  defp topic_error_none(topic_request) do
    %CreateTopics.TopicError{
      topic_name: topic_request.topic,
      error_code: :no_error
    }
  end

  defp topic_error_exists(topic_request) do
    %CreateTopics.TopicError{
      topic_name: topic_request.topic,
      error_code: :topic_already_exists
    }
  end

  defp produce_request(stream_name, partition, message, options) do
    topic = Kafka.topic(stream_name, options)
    partition = Kafka.partition(stream_name, partition, options)

    %Produce.Request{
      topic: topic,
      partition: partition,
      required_acks: 1,
      messages: [%Produce.Message{value: message}]
    }
  end

  defp fetch_options(_stream_name, index, limit, options) do
    [
      worker_name: Kafka.worker_name(options, @default_worker),
      offset: index,
      auto_commit: false,
      ## Assume messages are 1Kb or so.
      max_bytes: limit * 1024
    ]
  end

  defp fetch_options(fetch_options, last_index) do
    Keyword.put(fetch_options, :offset, last_index)
  end
end
