defmodule Ockam.Kafka.Interceptor.Protocol.Metadata.Request.Formatter do
  @moduledoc """
  Kafka protocol request formatter for metadata request

  Supported api versions 0-12
  """

  alias Ockam.Kafka.Interceptor.Protocol.Formatter, as: BaseFormatter
  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Request, as: MetadataRequest

  @api_metadata 3

  ## The difference between version 0 and 1 is not documented, but version 1 supports
  ## nullable arrays (of length -1)
  def format(%MetadataRequest{api_version: 0, topics: topics}) do
    BaseFormatter.format_type({:array, :string}, topics)
  end

  def format(%MetadataRequest{api_version: v1_3, topics: topics})
      when v1_3 <= 3 do
    BaseFormatter.format_type({:nullable_array, :string}, topics)
  end

  def format(%MetadataRequest{
        api_version: v4_7,
        topics: topics,
        allow_auto_topic_creation: allow_auto_topic_creation
      })
      when v4_7 <= 7 do
    BaseFormatter.format_typed_list([
      {{:nullable_array, :string}, topics},
      {:boolean, allow_auto_topic_creation}
    ])
  end

  def format(%MetadataRequest{
        api_version: 8,
        topics: topics,
        allow_auto_topic_creation: allow_auto_topic_creation,
        include_cluster_authorized_operations: include_cluster_authorized_operations,
        include_topic_authorized_operations: include_topic_authorized_operations
      }) do
    BaseFormatter.format_typed_list([
      {{:nullable_array, :string}, topics},
      {:boolean, allow_auto_topic_creation},
      {:boolean, include_cluster_authorized_operations},
      {:boolean, include_topic_authorized_operations}
    ])
  end

  def format(%MetadataRequest{
        api_version: v9_10 = api_version,
        topics: topics,
        allow_auto_topic_creation: allow_auto_topic_creation,
        include_cluster_authorized_operations: include_cluster_authorized_operations,
        include_topic_authorized_operations: include_topic_authorized_operations,
        tagged_fields: tagged_fields
      })
      when v9_10 <= 10 do
    BaseFormatter.format_typed_list([
      {{:compact_array, fn topic -> format_topic(api_version, topic) end}, topics},
      {:boolean, allow_auto_topic_creation},
      {:boolean, include_cluster_authorized_operations},
      {:boolean, include_topic_authorized_operations},
      {:tagged_fields, tagged_fields}
    ])
  end

  def format(%MetadataRequest{
        api_version: v11_12 = api_version,
        topics: topics,
        allow_auto_topic_creation: allow_auto_topic_creation,
        include_topic_authorized_operations: include_topic_authorized_operations,
        tagged_fields: tagged_fields
      })
      when v11_12 == 11 or v11_12 == 12 do
    BaseFormatter.format_typed_list([
      {{:compact_array, fn topic -> format_topic(api_version, topic) end}, topics},
      {:boolean, allow_auto_topic_creation},
      {:boolean, include_topic_authorized_operations},
      {:tagged_fields, tagged_fields}
    ])
  end

  def format(%MetadataRequest{api_version: other}) do
    {:error, {:unsupported_api, {@api_metadata, other}}}
  end

  def format(other) do
    {:error, {:not_metadata_request, other}}
  end

  defp format_topic(9, %MetadataRequest.Topic{
         name: name,
         tagged_fields: tagged_fields
       }) do
    BaseFormatter.format_typed_list([{:compact_string, name}, {:tagged_fields, tagged_fields}])
  end

  defp format_topic(v10_12, %MetadataRequest.Topic{
         name: name,
         topic_id: topic_id,
         tagged_fields: tagged_fields
       })
       when v10_12 >= 10 and v10_12 <= 12 do
    BaseFormatter.format_typed_list([
      {:uuid, topic_id},
      {:compact_nullable_string, name},
      {:tagged_fields, tagged_fields}
    ])
  end
end
