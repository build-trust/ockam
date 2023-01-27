defmodule Ockam.Kafka.Interceptor.Protocol.Metadata.Response.Formatter do
  @moduledoc """
  Kafka protocol response formatter for metadata response

  Supported api versions 0-12
  """

  alias Ockam.Kafka.Interceptor.Protocol.Formatter, as: BaseFormatter
  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response, as: MetadataResponse

  alias MetadataResponse.Broker
  alias MetadataResponse.Topic
  alias MetadataResponse.Topic.Partition

  @api_metadata 3

  ## The difference between version 0 and 1 is not documented, but version 1 supports
  ## nullable arrays (of length -1)
  def format(%MetadataResponse{api_version: 0 = api_version, brokers: brokers, topics: topics}) do
    BaseFormatter.format_typed_list([
      {{:array, fn broker -> format_broker(api_version, broker) end}, brokers},
      {{:array, fn topic -> format_topic(api_version, topic) end}, topics}
    ])
  end

  def format(%MetadataResponse{
        api_version: 1 = api_version,
        brokers: brokers,
        controller_id: controller_id,
        topics: topics
      }) do
    BaseFormatter.format_typed_list([
      {{:array, fn broker -> format_broker(api_version, broker) end}, brokers},
      {:int32, controller_id},
      {{:array, fn topic -> format_topic(api_version, topic) end}, topics}
    ])
  end

  def format(%MetadataResponse{
        api_version: 2 = api_version,
        brokers: brokers,
        cluster_id: cluster_id,
        controller_id: controller_id,
        topics: topics
      }) do
    BaseFormatter.format_typed_list([
      {{:array, fn broker -> format_broker(api_version, broker) end}, brokers},
      {:nullable_string, cluster_id},
      {:int32, controller_id},
      {{:array, fn topic -> format_topic(api_version, topic) end}, topics}
    ])
  end

  def format(%MetadataResponse{
        api_version: v3_7 = api_version,
        throttle_time_ms: throttle_time_ms,
        brokers: brokers,
        cluster_id: cluster_id,
        controller_id: controller_id,
        topics: topics
      })
      when v3_7 >= 3 and v3_7 <= 7 do
    BaseFormatter.format_typed_list([
      {:int32, throttle_time_ms},
      {{:array, fn broker -> format_broker(api_version, broker) end}, brokers},
      {:nullable_string, cluster_id},
      {:int32, controller_id},
      {{:array, fn topic -> format_topic(api_version, topic) end}, topics}
    ])
  end

  def format(%MetadataResponse{
        api_version: 8 = api_version,
        throttle_time_ms: throttle_time_ms,
        brokers: brokers,
        cluster_id: cluster_id,
        controller_id: controller_id,
        topics: topics,
        cluster_authorized_operations: cluster_authorized_operations
      }) do
    BaseFormatter.format_typed_list([
      {:int32, throttle_time_ms},
      {{:array, fn broker -> format_broker(api_version, broker) end}, brokers},
      {:nullable_string, cluster_id},
      {:int32, controller_id},
      {{:array, fn topic -> format_topic(api_version, topic) end}, topics},
      {:int32, cluster_authorized_operations}
    ])
  end

  def format(%MetadataResponse{
        api_version: v9_10 = api_version,
        throttle_time_ms: throttle_time_ms,
        brokers: brokers,
        cluster_id: cluster_id,
        controller_id: controller_id,
        topics: topics,
        cluster_authorized_operations: cluster_authorized_operations,
        tagged_fields: tagged_fields
      })
      when v9_10 == 9 or v9_10 == 10 do
    BaseFormatter.format_typed_list([
      {:int32, throttle_time_ms},
      {{:compact_array, fn broker -> format_broker(api_version, broker) end}, brokers},
      {:compact_nullable_string, cluster_id},
      {:int32, controller_id},
      {{:compact_array, fn topic -> format_topic(api_version, topic) end}, topics},
      {:int32, cluster_authorized_operations},
      {:tagged_fields, tagged_fields}
    ])
  end

  def format(%MetadataResponse{
        api_version: v11_12 = api_version,
        throttle_time_ms: throttle_time_ms,
        brokers: brokers,
        cluster_id: cluster_id,
        controller_id: controller_id,
        topics: topics,
        tagged_fields: tagged_fields
      })
      when v11_12 == 11 or v11_12 == 12 do
    BaseFormatter.format_typed_list([
      {:int32, throttle_time_ms},
      {{:compact_array, fn broker -> format_broker(api_version, broker) end}, brokers},
      {:compact_nullable_string, cluster_id},
      {:int32, controller_id},
      {{:compact_array, fn topic -> format_topic(api_version, topic) end}, topics},
      {:tagged_fields, tagged_fields}
    ])
  end

  def format(%MetadataResponse{api_version: other}) do
    {:error, {:unsupported_api, {@api_metadata, other}}}
  end

  defp format_broker(0, %Broker{node_id: node_id, host: host, port: port}) do
    BaseFormatter.format_typed_list([
      {:int32, node_id},
      {:string, host},
      {:int32, port}
    ])
  end

  defp format_broker(v1_8, %Broker{node_id: node_id, host: host, port: port, rack: rack})
       when v1_8 >= 1 and v1_8 <= 8 do
    BaseFormatter.format_typed_list([
      {:int32, node_id},
      {:string, host},
      {:int32, port},
      {:nullable_string, rack}
    ])
  end

  defp format_broker(v9_12, %Broker{
         node_id: node_id,
         host: host,
         port: port,
         rack: rack,
         tagged_fields: tagged_fields
       })
       when v9_12 >= 9 and v9_12 <= 12 do
    BaseFormatter.format_typed_list([
      {:int32, node_id},
      {:compact_string, host},
      {:int32, port},
      {:compact_nullable_string, rack},
      {:tagged_fields, tagged_fields}
    ])
  end

  defp format_topic(0 = api_version, %Topic{
         error_code: error_code,
         name: name,
         partitions: partitions
       }) do
    BaseFormatter.format_typed_list([
      {:int16, error_code},
      {:string, name},
      {{:array, fn partition -> format_partition(api_version, partition) end}, partitions}
    ])
  end

  defp format_topic(v1_7 = api_version, %Topic{
         error_code: error_code,
         name: name,
         is_internal: is_internal,
         partitions: partitions
       })
       when v1_7 >= 1 and v1_7 <= 7 do
    BaseFormatter.format_typed_list([
      {:int16, error_code},
      {:string, name},
      {:boolean, is_internal},
      {{:array, fn partition -> format_partition(api_version, partition) end}, partitions}
    ])
  end

  defp format_topic(8 = api_version, %Topic{
         error_code: error_code,
         name: name,
         is_internal: is_internal,
         partitions: partitions,
         topic_authorized_operations: topic_authorized_operations
       }) do
    BaseFormatter.format_typed_list([
      {:int16, error_code},
      {:string, name},
      {:boolean, is_internal},
      {{:array, fn partition -> format_partition(api_version, partition) end}, partitions},
      {:int32, topic_authorized_operations}
    ])
  end

  defp format_topic(9 = api_version, %Topic{
         error_code: error_code,
         name: name,
         is_internal: is_internal,
         partitions: partitions,
         topic_authorized_operations: topic_authorized_operations,
         tagged_fields: tagged_fields
       }) do
    BaseFormatter.format_typed_list([
      {:int16, error_code},
      {:compact_string, name},
      {:boolean, is_internal},
      {{:compact_array, fn partition -> format_partition(api_version, partition) end},
       partitions},
      {:int32, topic_authorized_operations},
      {:tagged_fields, tagged_fields}
    ])
  end

  defp format_topic(v10_11 = api_version, %Topic{
         error_code: error_code,
         name: name,
         topic_id: topic_id,
         is_internal: is_internal,
         partitions: partitions,
         topic_authorized_operations: topic_authorized_operations,
         tagged_fields: tagged_fields
       })
       when v10_11 == 10 or v10_11 == 11 do
    BaseFormatter.format_typed_list([
      {:int16, error_code},
      {:compact_string, name},
      {:uuid, topic_id},
      {:boolean, is_internal},
      {{:compact_array, fn partition -> format_partition(api_version, partition) end},
       partitions},
      {:int32, topic_authorized_operations},
      {:tagged_fields, tagged_fields}
    ])
  end

  defp format_topic(12 = api_version, %Topic{
         error_code: error_code,
         name: name,
         topic_id: topic_id,
         is_internal: is_internal,
         partitions: partitions,
         topic_authorized_operations: topic_authorized_operations,
         tagged_fields: tagged_fields
       }) do
    BaseFormatter.format_typed_list([
      {:int16, error_code},
      {:compact_nullable_string, name},
      {:uuid, topic_id},
      {:boolean, is_internal},
      {{:compact_array, fn partition -> format_partition(api_version, partition) end},
       partitions},
      {:int32, topic_authorized_operations},
      {:tagged_fields, tagged_fields}
    ])
  end

  defp format_partition(v0_4, %Partition{
         error_code: error_code,
         partition_index: partition_index,
         leader_id: leader_id,
         replica_nodes: replica_nodes,
         isr_nodes: isr_nodes
       })
       when v0_4 >= 0 and v0_4 <= 4 do
    BaseFormatter.format_typed_list([
      {:int16, error_code},
      {:int32, partition_index},
      {:int32, leader_id},
      {{:array, :int32}, replica_nodes},
      {{:array, :int32}, isr_nodes}
    ])
  end

  defp format_partition(v5_6, %Partition{
         error_code: error_code,
         partition_index: partition_index,
         leader_id: leader_id,
         replica_nodes: replica_nodes,
         isr_nodes: isr_nodes,
         offline_replicas: offline_replicas
       })
       when v5_6 == 5 or v5_6 == 6 do
    BaseFormatter.format_typed_list([
      {:int16, error_code},
      {:int32, partition_index},
      {:int32, leader_id},
      {{:array, :int32}, replica_nodes},
      {{:array, :int32}, isr_nodes},
      {{:array, :int32}, offline_replicas}
    ])
  end

  defp format_partition(v7_8, %Partition{
         error_code: error_code,
         partition_index: partition_index,
         leader_id: leader_id,
         leader_epoch: leader_epoch,
         replica_nodes: replica_nodes,
         isr_nodes: isr_nodes,
         offline_replicas: offline_replicas
       })
       when v7_8 == 7 or v7_8 == 8 do
    BaseFormatter.format_typed_list([
      {:int16, error_code},
      {:int32, partition_index},
      {:int32, leader_id},
      {:int32, leader_epoch},
      {{:array, :int32}, replica_nodes},
      {{:array, :int32}, isr_nodes},
      {{:array, :int32}, offline_replicas}
    ])
  end

  defp format_partition(v9_12, %Partition{
         error_code: error_code,
         partition_index: partition_index,
         leader_id: leader_id,
         leader_epoch: leader_epoch,
         replica_nodes: replica_nodes,
         isr_nodes: isr_nodes,
         offline_replicas: offline_replicas,
         tagged_fields: tagged_fields
       })
       when v9_12 >= 9 or v9_12 <= 12 do
    BaseFormatter.format_typed_list([
      {:int16, error_code},
      {:int32, partition_index},
      {:int32, leader_id},
      {:int32, leader_epoch},
      {{:compact_array, :int32}, replica_nodes},
      {{:compact_array, :int32}, isr_nodes},
      {{:compact_array, :int32}, offline_replicas},
      {:tagged_fields, tagged_fields}
    ])
  end
end
