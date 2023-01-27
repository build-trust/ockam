defmodule Ockam.Kafka.Interceptor.Protocol.Metadata.Response.Parser do
  @moduledoc """
  Kafka protocol response parser for metadata response

  Supported api versions 0-12
  """

  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response, as: MetadataResponse
  alias Ockam.Kafka.Interceptor.Protocol.Parser, as: BaseParser

  require Logger

  @spec parse(api_version :: integer(), data :: binary) ::
          {:ok, MetadataResponse.t(), rest :: binary()} | {:error, reason :: any()}
  def parse(0 = api_version, data) do
    with {:ok, brokers, rest} <- parse_brokers(api_version, data),
         {:ok, topics, rest} <- parse_topics(api_version, rest) do
      {:ok, %MetadataResponse{api_version: api_version, brokers: brokers, topics: topics}, rest}
    end
  end

  def parse(1 = api_version, data) do
    with {:ok, brokers, rest} <- parse_brokers(api_version, data),
         {:ok, controller_id, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, topics, rest} <- parse_topics(api_version, rest) do
      {:ok,
       %MetadataResponse{
         api_version: api_version,
         brokers: brokers,
         topics: topics,
         controller_id: controller_id
       }, rest}
    end
  end

  def parse(2 = api_version, data) do
    with {:ok, brokers, rest} <- parse_brokers(api_version, data),
         {:ok, cluster_id, rest} <- BaseParser.parse_type(:nullable_string, rest),
         {:ok, controller_id, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, topics, rest} <- parse_topics(api_version, rest) do
      {:ok,
       %MetadataResponse{
         api_version: api_version,
         brokers: brokers,
         topics: topics,
         controller_id: controller_id,
         cluster_id: cluster_id
       }, rest}
    end
  end

  def parse(v3_7 = api_version, data) when v3_7 >= 3 and v3_7 <= 7 do
    with {:ok, throttle_time_ms, rest} <- BaseParser.parse_type(:int32, data),
         {:ok, brokers, rest} <- parse_brokers(api_version, rest),
         {:ok, cluster_id, rest} <- BaseParser.parse_type(:nullable_string, rest),
         {:ok, controller_id, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, topics, rest} <- parse_topics(api_version, rest) do
      {:ok,
       %MetadataResponse{
         api_version: api_version,
         brokers: brokers,
         topics: topics,
         controller_id: controller_id,
         cluster_id: cluster_id,
         throttle_time_ms: throttle_time_ms
       }, rest}
    end
  end

  def parse(8 = api_version, data) do
    with {:ok, throttle_time_ms, rest} <- BaseParser.parse_type(:int32, data),
         {:ok, brokers, rest} <- parse_brokers(api_version, rest),
         {:ok, cluster_id, rest} <- BaseParser.parse_type(:nullable_string, rest),
         {:ok, controller_id, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, topics, rest} <- parse_topics(api_version, rest),
         {:ok, cluster_authorized_operations, rest} <- BaseParser.parse_type(:int32, rest) do
      {:ok,
       %MetadataResponse{
         api_version: api_version,
         brokers: brokers,
         topics: topics,
         controller_id: controller_id,
         cluster_id: cluster_id,
         throttle_time_ms: throttle_time_ms,
         cluster_authorized_operations: cluster_authorized_operations
       }, rest}
    end
  end

  def parse(v9_10 = api_version, data) when v9_10 == 9 or v9_10 == 10 do
    with {:ok, throttle_time_ms, rest} <- BaseParser.parse_type(:int32, data),
         {:ok, brokers, rest} <- parse_brokers(api_version, rest),
         {:ok, cluster_id, rest} <- BaseParser.parse_type(:compact_nullable_string, rest),
         {:ok, controller_id, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, topics, rest} <- parse_topics(api_version, rest),
         {:ok, cluster_authorized_operations, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok,
       %MetadataResponse{
         api_version: api_version,
         brokers: brokers,
         topics: topics,
         controller_id: controller_id,
         cluster_id: cluster_id,
         throttle_time_ms: throttle_time_ms,
         cluster_authorized_operations: cluster_authorized_operations,
         tagged_fields: tagged_fields
       }, rest}
    end
  end

  def parse(v11_12 = api_version, data) when v11_12 == 11 or v11_12 == 12 do
    with {:ok, throttle_time_ms, rest} <- BaseParser.parse_type(:int32, data),
         {:ok, brokers, rest} <- parse_brokers(api_version, rest),
         {:ok, cluster_id, rest} <- BaseParser.parse_type(:compact_nullable_string, rest),
         {:ok, controller_id, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, topics, rest} <- parse_topics(api_version, rest),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok,
       %MetadataResponse{
         api_version: api_version,
         brokers: brokers,
         topics: topics,
         controller_id: controller_id,
         cluster_id: cluster_id,
         throttle_time_ms: throttle_time_ms,
         tagged_fields: tagged_fields
       }, rest}
    end
  end

  def parse(api_version, _data) do
    {:error, {:metadata_response, :unsupported_version, api_version}}
  end

  defp parse_brokers(api_version, data) when api_version < 9 do
    BaseParser.parse_type({:array, fn data -> parse_broker(api_version, data) end}, data)
  end

  defp parse_brokers(api_version, data)
       when api_version >= 9 and api_version <= 12 do
    BaseParser.parse_type(
      {:compact_array, fn data -> parse_broker(api_version, data) end},
      data
    )
  end

  defp parse_topics(api_version, data) when api_version < 9 do
    BaseParser.parse_type({:array, fn data -> parse_topic(api_version, data) end}, data)
  end

  defp parse_topics(api_version, data)
       when api_version >= 9 and api_version <= 12 do
    BaseParser.parse_type(
      {:compact_array, fn data -> parse_topic(api_version, data) end},
      data
    )
  end

  defp parse_partitions(api_version, data) when api_version < 9 do
    BaseParser.parse_type(
      {:array, fn data -> parse_partition(api_version, data) end},
      data
    )
  end

  defp parse_partitions(api_version, data)
       when api_version >= 9 and api_version <= 12 do
    BaseParser.parse_type(
      {:compact_array, fn data -> parse_partition(api_version, data) end},
      data
    )
  end

  defp parse_broker(0, data) do
    with {:ok, node_id, rest} <- BaseParser.parse_type(:int32, data),
         {:ok, host, rest} <- BaseParser.parse_type(:string, rest),
         {:ok, port, rest} <- BaseParser.parse_type(:int32, rest) do
      {:ok, %MetadataResponse.Broker{node_id: node_id, host: host, port: port}, rest}
    end
  end

  defp parse_broker(v1_8, data) when v1_8 >= 1 and v1_8 <= 8 do
    with {:ok, node_id, rest} <- BaseParser.parse_type(:int32, data),
         {:ok, host, rest} <- BaseParser.parse_type(:string, rest),
         {:ok, port, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, rack, rest} <- BaseParser.parse_type(:nullable_string, rest) do
      {:ok, %MetadataResponse.Broker{node_id: node_id, host: host, port: port, rack: rack}, rest}
    end
  end

  defp parse_broker(v9_12, data) when v9_12 >= 9 and v9_12 <= 12 do
    with {:ok, node_id, rest} <- BaseParser.parse_type(:int32, data),
         {:ok, host, rest} <- BaseParser.parse_type(:compact_string, rest),
         {:ok, port, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, rack, rest} <- BaseParser.parse_type(:compact_nullable_string, rest),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok,
       %MetadataResponse.Broker{
         node_id: node_id,
         host: host,
         port: port,
         rack: rack,
         tagged_fields: tagged_fields
       }, rest}
    end
  end

  defp parse_topic(0 = api_version, data) do
    with {:ok, error_code, rest} <- BaseParser.parse_type(:int16, data),
         {:ok, name, rest} <- BaseParser.parse_type(:string, rest),
         {:ok, partitions, rest} <- parse_partitions(api_version, rest) do
      {:ok, %MetadataResponse.Topic{error_code: error_code, name: name, partitions: partitions},
       rest}
    end
  end

  defp parse_topic(v1_7 = api_version, data) when v1_7 >= 1 and v1_7 <= 7 do
    with {:ok, error_code, rest} <- BaseParser.parse_type(:int16, data),
         {:ok, name, rest} <- BaseParser.parse_type(:string, rest),
         {:ok, is_internal, rest} <- BaseParser.parse_type(:boolean, rest),
         {:ok, partitions, rest} <- parse_partitions(api_version, rest) do
      {:ok,
       %MetadataResponse.Topic{
         error_code: error_code,
         name: name,
         is_internal: is_internal,
         partitions: partitions
       }, rest}
    end
  end

  defp parse_topic(8 = api_version, data) do
    with {:ok, error_code, rest} <- BaseParser.parse_type(:int16, data),
         {:ok, name, rest} <- BaseParser.parse_type(:string, rest),
         {:ok, is_internal, rest} <- BaseParser.parse_type(:boolean, rest),
         {:ok, partitions, rest} <- parse_partitions(api_version, rest),
         {:ok, topic_authorized_operations, rest} <- BaseParser.parse_type(:int32, rest) do
      {:ok,
       %MetadataResponse.Topic{
         error_code: error_code,
         name: name,
         is_internal: is_internal,
         partitions: partitions,
         topic_authorized_operations: topic_authorized_operations
       }, rest}
    end
  end

  defp parse_topic(9 = api_version, data) do
    with {:ok, error_code, rest} <- BaseParser.parse_type(:int16, data),
         {:ok, name, rest} <- BaseParser.parse_type(:compact_string, rest),
         {:ok, is_internal, rest} <- BaseParser.parse_type(:boolean, rest),
         {:ok, partitions, rest} <- parse_partitions(api_version, rest),
         {:ok, topic_authorized_operations, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok,
       %MetadataResponse.Topic{
         error_code: error_code,
         name: name,
         is_internal: is_internal,
         partitions: partitions,
         topic_authorized_operations: topic_authorized_operations,
         tagged_fields: tagged_fields
       }, rest}
    end
  end

  defp parse_topic(v10_11 = api_version, data)
       when v10_11 == 10 or v10_11 == 11 do
    with {:ok, error_code, rest} <- BaseParser.parse_type(:int16, data),
         {:ok, name, rest} <- BaseParser.parse_type(:compact_string, rest),
         {:ok, topic_id, rest} <- BaseParser.parse_type(:uuid, rest),
         {:ok, is_internal, rest} <- BaseParser.parse_type(:boolean, rest),
         {:ok, partitions, rest} <- parse_partitions(api_version, rest),
         {:ok, topic_authorized_operations, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok,
       %MetadataResponse.Topic{
         error_code: error_code,
         name: name,
         topic_id: topic_id,
         is_internal: is_internal,
         partitions: partitions,
         topic_authorized_operations: topic_authorized_operations,
         tagged_fields: tagged_fields
       }, rest}
    end
  end

  defp parse_topic(12 = api_version, data) do
    with {:ok, error_code, rest} <- BaseParser.parse_type(:int16, data),
         {:ok, name, rest} <- BaseParser.parse_type(:compact_nullable_string, rest),
         {:ok, topic_id, rest} <- BaseParser.parse_type(:uuid, rest),
         {:ok, is_internal, rest} <- BaseParser.parse_type(:boolean, rest),
         {:ok, partitions, rest} <- parse_partitions(api_version, rest),
         {:ok, topic_authorized_operations, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok,
       %MetadataResponse.Topic{
         error_code: error_code,
         name: name,
         topic_id: topic_id,
         is_internal: is_internal,
         partitions: partitions,
         topic_authorized_operations: topic_authorized_operations,
         tagged_fields: tagged_fields
       }, rest}
    end
  end

  defp parse_partition(v0_4, data) when v0_4 >= 0 and v0_4 <= 4 do
    with {:ok, error_code, rest} <- BaseParser.parse_type(:int16, data),
         {:ok, partition_index, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, leader_id, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, replica_nodes, rest} <- BaseParser.parse_type({:array, :int32}, rest),
         {:ok, isr_nodes, rest} <- BaseParser.parse_type({:array, :int32}, rest) do
      {:ok,
       %MetadataResponse.Topic.Partition{
         error_code: error_code,
         partition_index: partition_index,
         leader_id: leader_id,
         replica_nodes: replica_nodes,
         isr_nodes: isr_nodes
       }, rest}
    end
  end

  defp parse_partition(v5_6, data) when v5_6 == 5 or v5_6 == 6 do
    with {:ok, error_code, rest} <- BaseParser.parse_type(:int16, data),
         {:ok, partition_index, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, leader_id, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, replica_nodes, rest} <- BaseParser.parse_type({:array, :int32}, rest),
         {:ok, isr_nodes, rest} <- BaseParser.parse_type({:array, :int32}, rest),
         {:ok, offline_replicas, rest} <- BaseParser.parse_type({:array, :int32}, rest) do
      {:ok,
       %MetadataResponse.Topic.Partition{
         error_code: error_code,
         partition_index: partition_index,
         leader_id: leader_id,
         replica_nodes: replica_nodes,
         isr_nodes: isr_nodes,
         offline_replicas: offline_replicas
       }, rest}
    end
  end

  defp parse_partition(v7_8, data) when v7_8 == 7 or v7_8 == 8 do
    with {:ok, error_code, rest} <- BaseParser.parse_type(:int16, data),
         {:ok, partition_index, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, leader_id, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, leader_epoch, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, replica_nodes, rest} <- BaseParser.parse_type({:array, :int32}, rest),
         {:ok, isr_nodes, rest} <- BaseParser.parse_type({:array, :int32}, rest),
         {:ok, offline_replicas, rest} <- BaseParser.parse_type({:array, :int32}, rest) do
      {:ok,
       %MetadataResponse.Topic.Partition{
         error_code: error_code,
         partition_index: partition_index,
         leader_id: leader_id,
         leader_epoch: leader_epoch,
         replica_nodes: replica_nodes,
         isr_nodes: isr_nodes,
         offline_replicas: offline_replicas
       }, rest}
    end
  end

  defp parse_partition(v9_12, data) when v9_12 >= 9 or v9_12 <= 12 do
    with {:ok, error_code, rest} <- BaseParser.parse_type(:int16, data),
         {:ok, partition_index, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, leader_id, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, leader_epoch, rest} <- BaseParser.parse_type(:int32, rest),
         {:ok, replica_nodes, rest} <- BaseParser.parse_type({:compact_array, :int32}, rest),
         {:ok, isr_nodes, rest} <- BaseParser.parse_type({:compact_array, :int32}, rest),
         {:ok, offline_replicas, rest} <- BaseParser.parse_type({:compact_array, :int32}, rest),
         {:ok, tagged_fields, rest} <- BaseParser.parse_type(:tagged_fields, rest) do
      {:ok,
       %MetadataResponse.Topic.Partition{
         error_code: error_code,
         partition_index: partition_index,
         leader_id: leader_id,
         leader_epoch: leader_epoch,
         replica_nodes: replica_nodes,
         isr_nodes: isr_nodes,
         offline_replicas: offline_replicas,
         tagged_fields: tagged_fields
       }, rest}
    end
  end
end
