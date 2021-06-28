defmodule Ockam.Stream.Storage.Kafka do
  @moduledoc """
    Kafka storage backend for ockam stream service
  """
  @behaviour Ockam.Stream.Storage

  alias Ockam.Kafka

  require Logger

  defstruct [:client, :options]

  @type options() :: Keyword.t()
  @type state() :: %__MODULE__{}
  @type message() :: %{index: integer(), data: binary()}

  ## Storage API
  @impl true
  @spec init_stream(String.t(), integer(), options()) :: {:ok, state()} | {:error, any()}
  ## TODO: call this from stream service
  def init_stream(stream_name, partitions, options) do
    Logger.debug("Init stream #{inspect(stream_name)} #{partitions}")

    topic = Kafka.topic(stream_name, options)

    with :ok <- Kafka.create_topic(topic, partitions, options) do
      {:ok, %__MODULE__{options: options}}
    end
  end

  @impl true
  @spec init_partition(String.t(), integer(), state(), options()) ::
          {:ok, options()} | {:error, any()}
  def init_partition(stream_name, partition, state, options) do
    options = Keyword.merge(state.options, options)

    client_id = Kafka.generate_client_id(stream_name, partition, options)

    with {:ok, client} <- Kafka.create_client(options, client_id) do
      {:ok, %__MODULE__{client: client, options: options}}
    end
  end

  @impl true
  @spec save(String.t(), integer(), binary(), state()) ::
          {{:ok, integer()} | {:error, any()}, state()}
  def save(stream_name, partition, message, state) do
    %__MODULE__{options: options, client: client} = state
    Logger.debug("Save #{inspect(stream_name)} #{partition}")
    topic = Kafka.topic(stream_name, options)
    partition = Kafka.partition(stream_name, partition, options)

    ## TODO: keys in stream protocol?
    key = ""

    result =
      case :brod.produce_sync_offset(client, topic, partition, key, message) do
        {:ok, offset} -> {:ok, offset}
        {:error, err} -> {:error, err}
      end

    {result, state}
  end

  @impl true
  @spec fetch(String.t(), integer(), integer(), integer(), state()) ::
          {{:ok, [message()]} | {:error, any()}, state()}
  def fetch(stream_name, partition, index, limit, state) do
    options = state.options
    Logger.debug("Fetch #{stream_name} #{partition}, #{index}")
    topic = Kafka.topic(stream_name, options)
    partition = Kafka.partition(stream_name, partition, options)

    result = fetch_messages(limit, topic, partition, index, state)

    {result, state}
  end

  defp fetch_messages(limit, topic, partition, offset, state, previous \\ []) do
    %__MODULE__{
      client: client,
      options: options
    } = state

    prev_count = Enum.count(previous)

    fetch_options = fetch_options(limit, options)

    ## TODO: optimize recursion with connection
    with {:ok, conn} <- :brod_client.get_leader_connection(client, topic, partition),
         {:ok, {_hw_offset, fetch_messages}} <-
           :brod.fetch(conn, topic, partition, offset, fetch_options) do
      messages =
        Enum.map(fetch_messages, fn message ->
          %{index: elem(message, 1), data: elem(message, 3)}
        end)

      case Enum.count(messages) do
        0 ->
          {:ok, previous}

        num when num + prev_count >= limit ->
          {:ok, previous ++ Enum.take(messages, limit)}

        _num ->
          last_index = last_index(messages)
          fetch_messages(limit, topic, partition, last_index + 1, state, previous ++ messages)
      end
    end
  end

  defp fetch_options(limit, _options) do
    %{
      # max_wait_time => wait()
      # , min_bytes => count()
      max_wait_time: 1,
      max_bytes: limit * 1024
      # isolation_level => isolation_level()
      # session_id => kpro:int32()
      # epoch => kpro:int32()
    }
  end

  defp last_index(messages) do
    messages
    |> Enum.map(fn %{index: index} -> index end)
    |> Enum.max()
  end
end
