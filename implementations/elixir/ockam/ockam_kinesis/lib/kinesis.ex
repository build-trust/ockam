defmodule Ockam.Kinesis do
  @moduledoc """
  Wrapper around AWS requests
  """
  require Logger

  alias Ockam.Kinesis.Config

  @type describe_stream_option() ::
          {:exclusive_start_shard_id, String.t() | nil} | {:limit, pos_integer()}
  @type put_record_option() ::
          {:explicit_hash_key, String.t()} | {:sequence_number_for_ordering, String.t()}
  @type shard_iterator_type() ::
          :after_sequence_number | :at_sequence_number | :at_timestamp | :latest | :trim_horizon
  @type shard_iterator_option() :: {:timestamp, float()} | {:starting_sequence_number, String.t()}

  @doc """
  Creates a Kinesis data stream.
  """
  @spec create_stream(stream_name :: String.t(), shards :: pos_integer()) :: :ok | {:error, any()}
  def create_stream(stream_name, shards) do
    request_body = %{"StreamName" => stream_name, "ShardCount" => shards}

    client()
    |> AWS.Kinesis.create_stream(request_body)
    |> handle_response()
    |> case do
      {:ok, _response} ->
        :ok

      {:error, %{"__type" => "ResourceInUseException"}} ->
        :ok

      error ->
        error
    end
  end

  @doc """
  Describes the specified Kinesis data stream.
  """
  @spec describe_stream(stream_name :: String.t(), options :: [describe_stream_option()]) ::
          {:ok, map()} | {:error, any()}
  def describe_stream(stream_name, options \\ []) do
    request_body = %{"StreamName" => stream_name}

    request_body =
      Enum.reduce(options, request_body, fn
        {:exclusive_start_shard_id, exclusive_start_shard_id}, acc ->
          Map.put(acc, "ExclusiveStartShardId", exclusive_start_shard_id)

        {:limit, limit}, acc ->
          Map.put(acc, "Limit", limit)

        _other, acc ->
          acc
      end)

    client()
    |> AWS.Kinesis.describe_stream(request_body)
    |> handle_response()
  end

  @doc """
  Provides a summarized description of the specified Kinesis data stream without the shard list.
  """
  @spec describe_stream(stream_name :: String.t()) :: {:ok, map()} | {:error, any()}
  def describe_stream_summary(stream_name) do
    request_body = %{"StreamName" => stream_name}

    client()
    |> AWS.Kinesis.describe_stream_summary(request_body)
    |> handle_response()
  end

  @doc """
  Gets an Amazon Kinesis shard iterator.
  """
  @spec get_shard_iterator(
          stream_name :: String.t(),
          partition :: non_neg_integer(),
          type :: shard_iterator_type(),
          options :: [shard_iterator_option()]
        ) :: {:ok, String.t()} | {:error, any()}
  def get_shard_iterator(stream_name, partition, type, options \\ []) do
    shard_id = shard_id(partition)

    request_body = %{
      "ShardId" => shard_id,
      "ShardIteratorType" => type |> Atom.to_string() |> String.upcase(),
      "StreamName" => stream_name
    }

    request_body =
      Enum.reduce(options, request_body, fn
        {:timestamp, timestamp}, acc ->
          Map.put(acc, "Timestamp", timestamp)

        {:starting_sequence_number, starting_sequence_number}, acc ->
          Map.put(acc, "StartingSequenceNumber", starting_sequence_number)

        _other, acc ->
          acc
      end)

    client()
    |> AWS.Kinesis.get_shard_iterator(request_body)
    |> handle_response()
    |> case do
      {:ok, %{"ShardIterator" => shard_iterator}} ->
        {:ok, shard_iterator}

      error ->
        error
    end
  end

  @doc """
  Gets data records from a Kinesis data stream's shard.
  """
  @spec get_records(shard_iterator :: String.t(), limit :: pos_integer()) ::
          {:ok,
           {[%{index: non_neg_integer(), data: binary()}], String.t() | nil, pos_integer() | nil}}
          | {:error, any()}
  def get_records(shard_iterator, limit \\ 1) do
    request_body = %{"ShardIterator" => shard_iterator, "Limit" => limit}

    client()
    |> AWS.Kinesis.get_records(request_body)
    |> handle_response()
    |> case do
      # NOTE: MillisBehindLatest being 0 indicates that the end of the stream has been reached
      {:ok,
       %{"Records" => [], "MillisBehindLatest" => 0, "NextShardIterator" => next_shard_iterator}} ->
        {:ok, {[], next_shard_iterator, nil}}

      {:ok, %{"Records" => [], "NextShardIterator" => next_shard_iterator}} ->
        get_records(next_shard_iterator, limit)

      {:ok, %{"Records" => records}} ->
        {records, last_index} =
          Enum.reduce(records, {[], nil}, fn %{"Data" => data, "SequenceNumber" => index},
                                             {records, _last_index} ->
            index = String.to_integer(index)
            {[%{index: index, data: :base64.decode(data)} | records], index}
          end)

        records = Enum.reverse(records)

        {:ok, {records, nil, last_index}}

      error ->
        error
    end
  end

  @doc """
  Writes a single data record into an Amazon Kinesis data stream.
  """
  @spec put_record(
          stream_name :: String.t(),
          message :: binary(),
          partition_key :: String.t() | nil,
          options :: [put_record_option()]
        ) ::
          {:ok, String.t()} | {:error, any()}
  def put_record(stream_name, message, partition_key \\ nil, options \\ []) do
    request_body = %{
      "Data" => Base.encode64(message),
      "PartitionKey" => partition_key || random_partition_key(),
      "StreamName" => stream_name
    }

    request_body =
      Enum.reduce(options, request_body, fn
        {:explicit_hash_key, explicit_hash_key}, acc ->
          Map.put(acc, "ExplicitHashKey", explicit_hash_key)

        {:sequence_number_for_ordering, sequence_number_for_ordering}, acc ->
          Map.put(acc, "SequenceNumberForOrdering", sequence_number_for_ordering)

        _other, acc ->
          acc
      end)

    client()
    |> AWS.Kinesis.put_record(request_body)
    |> handle_response()
    |> case do
      {:ok, %{"SequenceNumber" => sequence_number}} ->
        {:ok, sequence_number}

      {:error, reason} = error ->
        Logger.error("Error putting record: #{inspect(reason)}")
        error
    end
  end

  @doc """
  Converts an integer to a shard id
  """
  @spec shard_id(partition :: integer()) :: String.t() | nil
  def shard_id(partition) when is_integer(partition) and partition < 0, do: nil

  def shard_id(partition) when is_integer(partition) do
    partition
    |> to_string()
    |> String.pad_leading(20, "shardId-000000000000")
  end

  defp handle_response({:ok, response, _http_response}), do: {:ok, response}

  defp handle_response({:error, {:unexpected_response, %{body: response_body}}}) do
    {:error, Jason.decode!(response_body)}
  end

  defp handle_response({:error, _reason} = error), do: error

  defp random_partition_key() do
    20 |> :crypto.strong_rand_bytes() |> Base.encode32(case: :lower, padding: false)
  end

  defp client do
    %AWS.Client{
      access_key_id: Config.access_key_id(),
      region: Config.region(),
      secret_access_key: Config.secret_access_key(),
      http_client: Config.http_client()
    }
  end
end
