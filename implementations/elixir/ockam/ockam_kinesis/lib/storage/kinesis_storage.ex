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
end
