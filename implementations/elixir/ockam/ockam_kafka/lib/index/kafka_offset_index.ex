defmodule Ockam.Stream.Index.KafkaOffset do
  @moduledoc """
    Kafka storage backend for ockam stream index service
    Using kafka offset storage
  """
  @behaviour Ockam.Stream.Index.Storage

  alias Ockam.Kafka

  require Logger

  defstruct [:options, coordinators: %{}]

  @impl true
  def init(options) do
    {:ok, %__MODULE__{options: options}}
  end

  @impl true
  def get_index(client_id, stream_name, partition, state) do
    Logger.debug("Get index start #{inspect(client_id)}")
    %__MODULE__{options: options} = state
    topic = Kafka.topic(stream_name, options)
    partition = Kafka.partition(stream_name, partition, options)
    consumer_id = Kafka.consumer_id(client_id, options)

    Logger.debug("Get index request")
    Logger.debug("Options: #{inspect(options)}")

    fetch_committed_offsets(consumer_id, topic, partition, state)
  end

  @impl true
  def save_index(client_id, stream_name, partition, index, state) do
    %__MODULE__{options: options} = state
    consumer_id = Kafka.consumer_id(client_id, options)
    topic = Kafka.topic(stream_name, options)
    partition = Kafka.partition(stream_name, partition, options)

    commit_offset(consumer_id, topic, partition, index, state)
  end

  def commit_offset(consumer_id, topic, partition, index, state) do
    %__MODULE__{options: options} = state

    req_body = [
      group_id: consumer_id,
      generation_id: -1,
      member_id: "",
      retention_time: -1,
      topics: [
        [
          topic: topic,
          partitions: [
            [
              partition: partition,
              offset: index,
              metadata: ""
            ]
          ]
        ]
      ]
    ]

    with_coordinator(consumer_id, state, fn coordinator ->
      req = :brod_kafka_request.offset_commit(coordinator, req_body)

      case :brod_utils.request_sync(coordinator, req, request_timeout(options)) do
        {:ok,
         %{
           responses: [
             %{
               partition_responses: [%{error_code: :no_error, partition: ^partition}],
               topic: ^topic
             }
           ]
         }} ->
          :ok

        {:ok, %{responses: [%{partition_responses: [%{error_code: error}]}]}} ->
          {:error, {:commit_error, error}}
      end
    end)
  end

  def fetch_committed_offsets(consumer_id, topic, partition, state) do
    with_coordinator(consumer_id, state, fn coordinator ->
      req = :brod_kafka_request.offset_fetch(coordinator, consumer_id, [{topic, [partition]}])

      case :brod_utils.request_sync(coordinator, req) do
        {:ok, msg} ->
          [
            %{
              partition_responses: [
                %{error_code: :no_error, metadata: _, offset: offset, partition: ^partition}
              ],
              topic: ^topic
            }
          ] = :kpro.find(:responses, msg)

          offset =
            case offset do
              -1 -> :undefined
              non_neg when non_neg >= 0 -> non_neg
            end

          {:ok, offset}

        {:error, reason} ->
          {:error, {:fetch_offset_error, reason}}
      end
    end)
  end

  def with_coordinator(consumer_id, state, fun) do
    {:ok, coordinator} =
      case get_coordinator(consumer_id, state) do
        {:ok, coordinator} ->
          refresh_coordinator(coordinator, consumer_id, state)

        :error ->
          connect_coordinator(consumer_id, state)
      end

    state = update_coordinator(coordinator, consumer_id, state)
    {fun.(coordinator), state}
  end

  def get_coordinator(consumer_id, state) do
    Map.fetch(state.coordinators, consumer_id)
  end

  def refresh_coordinator(coordinator, consumer_id, state) do
    %__MODULE__{options: options} = state
    client_config = Kafka.client_config(options)
    timeout = Keyword.get(client_config, :connect_timeout, 20_000)

    {:ok, ep} = :kpro_connection.get_endpoint(coordinator)

    case :kpro_brokers.discover_coordinator(coordinator, :group, consumer_id, timeout) do
      {:ok, ^ep} ->
        {:ok, coordinator}

      {:ok, _other} ->
        connect_coordinator(consumer_id, state)

      other ->
        other
    end
  end

  def connect_coordinator(consumer_id, state) do
    %__MODULE__{options: options} = state
    endpoints = Kafka.endpoints(options)
    client_config = Kafka.client_config(options)
    timeout = Keyword.get(client_config, :connect_timeout, 20_000)

    args = %{type: :group, id: consumer_id, timeout: Keyword.get(options, :timeout, timeout)}
    :kpro.connect_coordinator(endpoints, client_config, args)
  end

  def update_coordinator(coordinator, consumer_id, state) do
    %{coordinators: coordinators} = state
    %{state | coordinators: Map.put(coordinators, consumer_id, coordinator)}
  end

  def request_timeout(options) do
    options |> Kafka.request_configs() |> Map.fetch!(:timeout)
  end
end
