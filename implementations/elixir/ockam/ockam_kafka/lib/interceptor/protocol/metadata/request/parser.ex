defmodule Ockam.Kafka.Interceptor.Protocol.Metadata.Request.Parser do
  @moduledoc """
  Kafka protocol request parser for metadata request

  Supported api versions 0-12
  """

  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Request, as: MetadataRequest
  alias Ockam.Kafka.Interceptor.Protocol.Parser, as: BaseParser

  require Logger

  @spec parse(api_version :: integer(), data :: binary) ::
          {:ok, MetadataRequest.t(), rest :: binary()} | {:error, reason :: any()}
  ## The difference between version 0 and 1 is not documented, but version 1 supports
  ## nullable arrays (of length -1)
  def parse(0 = api_version, data) do
    with {:ok, topics, rest} <- BaseParser.parse_type({:array, :string}, data) do
      {:ok, %MetadataRequest{api_version: api_version, topics: topics}, rest}
    end
  end

  def parse(v1_3 = api_version, data) when v1_3 <= 3 do
    with {:ok, topics, rest} <- BaseParser.parse_type({:nullable_array, :string}, data) do
      {:ok, %MetadataRequest{api_version: api_version, topics: topics}, rest}
    end
  end

  def parse(v4_7 = api_version, data) when v4_7 <= 7 do
    with {:ok, topics, rest} <- BaseParser.parse_type({:nullable_array, :string}, data),
         {:ok, allow_auto_topic_creation, rest} <- BaseParser.parse_type(:boolean, rest) do
      {:ok,
       %MetadataRequest{
         api_version: api_version,
         topics: topics,
         allow_auto_topic_creation: allow_auto_topic_creation
       }, rest}
    end
  end

  def parse(8 = api_version, data) do
    with {:ok, topics, rest} <- BaseParser.parse_type({:nullable_array, :string}, data),
         {:ok, allow_auto_topic_creation, rest} <- BaseParser.parse_type(:boolean, rest),
         {:ok, include_cluster_authorized_operations, rest} <-
           BaseParser.parse_type(:boolean, rest),
         {:ok, include_topic_authorized_operations, rest} <- BaseParser.parse_type(:boolean, rest) do
      {:ok,
       %MetadataRequest{
         api_version: api_version,
         topics: topics,
         allow_auto_topic_creation: allow_auto_topic_creation,
         include_cluster_authorized_operations: include_cluster_authorized_operations,
         include_topic_authorized_operations: include_topic_authorized_operations
       }, rest}
    end
  end

  def parse(v9_10 = api_version, data) when v9_10 <= 10 do
    with {:ok, topics, rest} <-
           BaseParser.parse_type(
             {:compact_array, fn data -> parse_topic(api_version, data) end},
             data
           ),
         {:ok, allow_auto_topic_creation, rest} <- BaseParser.parse_type(:boolean, rest),
         {:ok, include_cluster_authorized_operations, rest} <-
           BaseParser.parse_type(:boolean, rest),
         {:ok, include_topic_authorized_operations, rest} <-
           BaseParser.parse_type(:boolean, rest),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok,
       %MetadataRequest{
         api_version: api_version,
         topics: topics,
         allow_auto_topic_creation: allow_auto_topic_creation,
         include_cluster_authorized_operations: include_cluster_authorized_operations,
         include_topic_authorized_operations: include_topic_authorized_operations,
         tagged_fields: tagged_fields
       }, rest}
    end
  end

  def parse(v11_12 = api_version, data) when v11_12 == 11 or v11_12 == 12 do
    with {:ok, topics, rest} <-
           BaseParser.parse_type(
             {:compact_array, fn data -> parse_topic(api_version, data) end},
             data
           ),
         {:ok, allow_auto_topic_creation, rest} <- BaseParser.parse_type(:boolean, rest),
         {:ok, include_topic_authorized_operations, rest} <-
           BaseParser.parse_type(:boolean, rest),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok,
       %MetadataRequest{
         api_version: api_version,
         topics: topics,
         allow_auto_topic_creation: allow_auto_topic_creation,
         include_topic_authorized_operations: include_topic_authorized_operations,
         tagged_fields: tagged_fields
       }, rest}
    end
  end

  def parse(api_version, _data) do
    {:error, {:metadata_request, :unsupported_version, api_version}}
  end

  defp parse_topic(9, data) do
    with {:ok, name, rest} <- BaseParser.parse_type(:compact_string, data),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok, %MetadataRequest.Topic{name: name, tagged_fields: tagged_fields}, rest}
    end
  end

  defp parse_topic(v10_12, data) when v10_12 >= 10 and v10_12 <= 12 do
    with {:ok, topic_id, rest} <- BaseParser.parse_type(:uuid, data),
         {:ok, name, rest} <- BaseParser.parse_type(:compact_nullable_string, rest),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok,
       %MetadataRequest.Topic{
         topic_id: topic_id,
         name: name,
         tagged_fields: tagged_fields
       }, rest}
    end
  end
end
