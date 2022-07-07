defmodule Ockam.Kinesis do
  @moduledoc """
  Wrapper around ExAws.Kinesis requests
  """
  require Logger

  @doc """
  Creates a Kinesis data stream.
  """
  @spec create_stream(stream_name :: String.t(), shards :: pos_integer()) :: :ok | {:error, any()}
  def create_stream(stream_name, shards) do
    stream_name
    |> ExAws.Kinesis.create_stream(shards)
    |> ExAws.request()
    |> case do
      {:ok, _response} ->
        :ok

      {:error, {"ResourceInUseException", _detail}} ->
        :ok

      {:error, reason} = error ->
        Logger.error("Error creating stream: #{inspect(reason)}")
        error
    end
  end

  @doc """
  Describes the specified Kinesis data stream.
  """
  @spec describe_stream(stream_name :: String.t(), options :: Keyword.t()) ::
          {:ok, map()} | {:error, any()}
  def describe_stream(stream_name, options \\ []) do
    stream_name
    |> ExAws.Kinesis.describe_stream(options)
    |> ExAws.request()
  end

  @doc """
  Provides a summarized description of the specified Kinesis data stream without the shard list.
  """
  @spec describe_stream(stream_name :: String.t()) :: {:ok, map()} | {:error, any()}
  def describe_stream_summary(stream_name) do
    stream_name
    |> ExAws.Kinesis.describe_stream_summary()
    |> ExAws.request()
  end

  @doc """
  Gets an Amazon Kinesis shard iterator.
  """
  @spec get_shard_iterator(
          stream_name :: String.t(),
          partition :: non_neg_integer(),
          type :: ExAws.Kinesis.shard_iterator_types(),
          options :: ExAws.Kinesis.get_shard_iterator_opts()
        ) :: {:ok, String.t()} | {:error, any()}
  def get_shard_iterator(stream_name, partition, type, opts \\ []) do
    shard_id = shard_id(partition)

    stream_name
    |> ExAws.Kinesis.get_shard_iterator(shard_id, type, opts)
    |> do_get_shard_iterator()
  end

  @doc """
  Gets data records from a Kinesis data stream's shard.
  """
  @spec get_records(shard_iterator :: String.t(), limit :: pos_integer()) ::
          {:ok,
           {[%{index: non_neg_integer(), data: binary()}], String.t() | nil, pos_integer() | nil}}
          | {:error, any()}
  def get_records(shard_iterator, limit \\ 1) do
    shard_iterator
    |> ExAws.Kinesis.get_records(limit: limit)
    |> ExAws.request()
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

      {:error, reason} = error ->
        Logger.error("Error getting records: #{inspect(reason)}")
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
          opts :: ExAws.Kinesis.put_record_opts()
        ) ::
          {:ok, String.t()} | {:error, any()}
  def put_record(stream_name, message, partition_key \\ nil, opts \\ []) do
    partition_key = partition_key || random_partition_key()

    stream_name
    |> ExAws.Kinesis.put_record(partition_key, message, opts)
    |> ExAws.request()
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

  defp do_get_shard_iterator(operation) do
    operation
    |> ExAws.request()
    |> case do
      {:ok, %{"ShardIterator" => shard_iterator}} ->
        {:ok, shard_iterator}

      {:error, reason} = error ->
        Logger.error("Error getting shard iterator: #{inspect(reason)}")
        error
    end
  end

  defp random_partition_key() do
    20 |> :crypto.strong_rand_bytes() |> Base.encode32(case: :lower, padding: false)
  end
end
