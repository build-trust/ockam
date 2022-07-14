defmodule Ockam.Stream.Storage.Kinesis do
  @moduledoc """
  AWS Kinesis stroage backend for Ockam stream service
  """
  @behaviour Ockam.Stream.Storage

  require Logger

  alias Ockam.Kinesis

  defmodule State do
    @moduledoc false

    defstruct [
      :hash_key,
      :initial_sequence_number,
      :previous_index,
      :previous_sequence_number,
      :next_shard_iterator,
      :options,
      :sequence_number_for_ordering
    ]

    @type t() :: %__MODULE__{
            hash_key: String.t() | nil,
            initial_sequence_number: pos_integer() | nil,
            next_shard_iterator: String.t(),
            options: Keyword.t(),
            previous_index: non_neg_integer() | nil,
            previous_sequence_number: non_neg_integer() | nil,
            sequence_number_for_ordering: String.t() | nil
          }
  end

  @type options() :: Keyword.t()
  @type state() :: State.t()
  @type message() :: %{index: integer(), data: binary()}

  @impl true
  @spec init_stream(String.t(), integer(), options()) :: {:ok, state()} | {:error, any()}
  def init_stream(stream_name, partitions, options) do
    Logger.debug("Init stream. stream_name: #{stream_name}, partitions: #{partitions}")

    with :ok <- Kinesis.create_stream(stream_name, partitions),
         :ok <- await_stream_activation(stream_name) do
      {:ok, %State{options: options}}
    end
  end

  @impl true
  @spec init_partition(String.t(), integer(), state(), options()) ::
          {:ok, state()} | {:error, any()}
  def init_partition(stream_name, partition, state, options) do
    Logger.debug(
      "Init partition. stream_name: #{stream_name}, partition: #{partition}, state: #{inspect(state)}"
    )

    options = Keyword.merge(state.options, options)

    # NOTE: We're retrieving shard hash key to ensure that producer will
    #       put messages to a particular shard.
    #       We're also storing initial sequence number to keep a reference to
    #       the beginning of the shard.
    previous_shard_id = Kinesis.shard_id(partition - 1)

    case Kinesis.describe_stream(stream_name,
           exclusive_start_shard_id: previous_shard_id,
           limit: 1
         ) do
      {:ok,
       %{
         "StreamDescription" => %{
           "Shards" => [
             %{
               "HashKeyRange" => %{"StartingHashKey" => hash_key},
               "SequenceNumberRange" => %{"StartingSequenceNumber" => initial_sequence_number}
             }
           ]
         }
       }} ->
        {:ok,
         %State{
           hash_key: hash_key,
           initial_sequence_number: String.to_integer(initial_sequence_number),
           options: options
         }}

      error ->
        error
    end
  end

  @impl true
  @spec save(String.t(), integer(), binary(), state()) ::
          {{:ok, integer()} | {:error, any()}, state()}
  def save(
        stream_name,
        partition,
        message,
        %State{
          hash_key: hash_key,
          sequence_number_for_ordering: sequence_number_for_ordering
        } = state
      ) do
    Logger.debug("Save. stream_name: #{stream_name}, partition: #{partition}")

    case Kinesis.put_record(stream_name, message, nil,
           explicit_hash_key: hash_key,
           sequence_number_for_ordering: sequence_number_for_ordering
         ) do
      {:ok, sequence_number} ->
        state = %{state | sequence_number_for_ordering: sequence_number}
        {{:ok, String.to_integer(sequence_number)}, state}

      error ->
        {error, state}
    end
  end

  @impl true
  @spec fetch(String.t(), integer(), integer(), integer(), state()) ::
          {{:ok, [message()]} | {:error, any()}, state()}
  def fetch(stream_name, partition, index, limit, state) do
    Logger.debug("Fetch. stream_name: #{stream_name}, partition: #{partition}, index: #{index}")

    fetch_messages(stream_name, partition, index, limit, state)
  end

  # NOTE: Kinesis stream gets initialized in CREATING status.
  #       We need to wait until the stream status is ACTIVE
  #       before it can be interacted with.
  #       Rate limit on DescribeStreamSummary calls is 20 per second.
  #       A retry limit could be added to avoid process hanging.
  defp await_stream_activation(stream_name) do
    case Kinesis.describe_stream_summary(stream_name) do
      {:ok, %{"StreamDescriptionSummary" => %{"StreamStatus" => "ACTIVE"}}} ->
        :ok

      {:ok, _response} ->
        :timer.sleep(50)
        await_stream_activation(stream_name)

      # NOTE: In case for some reason we hit the rate limit, we want to wait
      #       instead of timing out.
      {:error, {"LimitExceededException", _}} ->
        :timer.sleep(50)
        await_stream_activation(stream_name)

      error ->
        error
    end
  end

  defp fetch_messages(
         stream_name,
         partition,
         index,
         limit,
         %State{previous_sequence_number: previous_sequence_number} = state
       ) do
    with {:ok, shard_iterator} <- get_shard_iterator(stream_name, partition, index, state),
         {:ok, {records, next_shard_iterator, latest_index}} <-
           Kinesis.get_records(shard_iterator, limit) do
      {{:ok, records},
       %{
         state
         | previous_index: index,
           previous_sequence_number: latest_index || previous_sequence_number,
           next_shard_iterator: next_shard_iterator
       }}
    else
      {:error, {"ExpiredIteratorException", _}} ->
        state = %{state | next_shard_iterator: nil}
        fetch_messages(stream_name, partition, index, limit, state)

      error ->
        {error, state}
    end
  end

  # NOTE: When we repeat fetching messages for the same index and we have a shard
  #       iterator stored in the state, we pick up where we left off.
  #       This would clause would realistically get invoked only if the index
  #       is at the end of the stream and we're awaiting new messages.
  #       If an iterator has expired, we reset it to `nil` so it can be retrieved again.
  defp get_shard_iterator(_stream_name, _partition, index, %State{
         previous_index: index,
         next_shard_iterator: shard_iterator
       })
       when is_binary(shard_iterator) do
    {:ok, shard_iterator}
  end

  # NOTE: If the index is lower than initial sequence number, we start reading from
  #       the beginning of the stream
  defp get_shard_iterator(stream_name, partition, index, %State{
         initial_sequence_number: initial_sequence_number
       })
       when index <= initial_sequence_number do
    Kinesis.get_shard_iterator(stream_name, partition, :trim_horizon)
  end

  # NOTE: If the requested index is bigger than the last processed sequence number by 1
  #       we know to look for messages that come after the last proccessed message
  defp get_shard_iterator(
         stream_name,
         partition,
         index,
         %State{previous_sequence_number: previous_sequence_number}
       )
       when not is_nil(previous_sequence_number) and index - previous_sequence_number == 1 do
    Kinesis.get_shard_iterator(stream_name, partition, :after_sequence_number,
      starting_sequence_number: to_string(previous_sequence_number)
    )
  end

  # NOTE: In other case we simply try to retrieve messages at the current index
  defp get_shard_iterator(stream_name, partition, index, _state) do
    Kinesis.get_shard_iterator(stream_name, partition, :at_sequence_number,
      starting_sequence_number: to_string(index)
    )
  end
end
