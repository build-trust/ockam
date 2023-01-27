defmodule Ockam.Kafka.Interceptor.Protocol.Metadata.Response.Test do
  use ExUnit.Case

  alias Ockam.Kafka.Interceptor.Protocol.Formatter

  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response, as: MetadataResponse
  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response.Formatter, as: ResponseFormatter
  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Response.Parser, as: ResponseParser

  alias Ockam.Kafka.Interceptor.Protocol.Parser

  alias Ockam.Kafka.Interceptor.Protocol.RequestHeader
  alias Ockam.Kafka.Interceptor.Protocol.ResponseHeader

  @api_metadata 3

  test "parse/format metadata response body" do
    ## Only check that format/parse match

    Enum.each(0..12, fn api_version ->
      response = metadata_response(api_version)

      {:ok, formatted} = ResponseFormatter.format(response)

      {:ok, ^response, <<>>} = ResponseParser.parse(api_version, formatted)
    end)
  end

  test "parse full metadata response" do
    Enum.each(0..12, fn api_version ->
      {:ok, header_version} = Parser.response_header_version(@api_metadata, api_version)

      response_header = response_header(header_version, api_version)
      request_header = request_header(response_header)

      {:ok, header_binary} = Formatter.format_response_header(response_header)

      response = metadata_response(api_version)

      {:ok, formatted} = ResponseFormatter.format(response)
      size = byte_size(formatted)

      message = header_binary <> formatted

      assert {:ok, ^response_header, ^size, ^response} =
               Parser.parse_kafka_response_for_request(request_header, message)
    end)
  end

  def response_header(0, api_version) do
    %ResponseHeader{
      header_version: 0,
      api_version: api_version,
      api_key: @api_metadata,
      correlation_id: 10,
      client_id: "foo"
    }
  end

  def response_header(1, api_version) do
    %ResponseHeader{
      header_version: 1,
      api_version: api_version,
      api_key: @api_metadata,
      correlation_id: 10,
      client_id: "foo",
      tagged_fields: %{}
    }
  end

  def request_header(response_header) do
    struct(RequestHeader, Map.from_struct(response_header))
  end

  def metadata_response(api_version) when api_version == 0 do
    %MetadataResponse{
      api_version: api_version,
      brokers: brokers(api_version),
      topics: topics(api_version)
    }
  end

  def metadata_response(api_version) when api_version == 1 do
    %MetadataResponse{
      api_version: api_version,
      brokers: brokers(api_version),
      controller_id: 5,
      topics: topics(api_version)
    }
  end

  def metadata_response(api_version) when api_version == 2 do
    %MetadataResponse{
      api_version: api_version,
      brokers: brokers(api_version),
      cluster_id: "foo",
      controller_id: 5,
      topics: topics(api_version)
    }
  end

  def metadata_response(api_version) when api_version < 8 do
    %MetadataResponse{
      api_version: api_version,
      throttle_time_ms: 1000,
      brokers: brokers(api_version),
      cluster_id: "foo",
      controller_id: 5,
      topics: topics(api_version)
    }
  end

  def metadata_response(api_version) when api_version == 8 do
    %MetadataResponse{
      api_version: api_version,
      throttle_time_ms: 1000,
      brokers: brokers(api_version),
      cluster_id: "foo",
      controller_id: 5,
      topics: topics(api_version),
      cluster_authorized_operations: 0xFFFF
    }
  end

  def metadata_response(api_version) when api_version < 11 do
    %MetadataResponse{
      api_version: api_version,
      throttle_time_ms: 1000,
      brokers: brokers(api_version),
      cluster_id: "foo",
      controller_id: 5,
      topics: topics(api_version),
      cluster_authorized_operations: 0xFFFF,
      tagged_fields: %{}
    }
  end

  def metadata_response(api_version) do
    %MetadataResponse{
      api_version: api_version,
      throttle_time_ms: 1000,
      brokers: brokers(api_version),
      cluster_id: "foo",
      controller_id: 5,
      topics: topics(api_version),
      tagged_fields: %{}
    }
  end

  def brokers(api_version) when api_version < 9 do
    [
      %MetadataResponse.Broker{
        node_id: 0,
        host: "example.com",
        port: 123
      },
      %MetadataResponse.Broker{
        node_id: 1,
        host: "example1.com",
        port: 1234
      }
    ]
  end

  def brokers(_api_version) do
    [
      %MetadataResponse.Broker{
        node_id: 0,
        host: "example.com",
        port: 123,
        tagged_fields: %{}
      },
      %MetadataResponse.Broker{
        node_id: 1,
        host: "example1.com",
        port: 1234,
        tagged_fields: %{}
      }
    ]
  end

  def topics(api_version) when api_version == 0 do
    [
      %MetadataResponse.Topic{
        error_code: 0,
        name: "foo",
        partitions: partitions(api_version)
      }
    ]
  end

  def topics(api_version) when api_version < 8 do
    [
      %MetadataResponse.Topic{
        error_code: 0,
        name: "foo",
        is_internal: false,
        partitions: partitions(api_version)
      }
    ]
  end

  def topics(api_version) when api_version == 8 do
    [
      %MetadataResponse.Topic{
        error_code: 0,
        name: "foo",
        is_internal: false,
        topic_authorized_operations: 0xFF,
        partitions: partitions(api_version)
      }
    ]
  end

  def topics(api_version) when api_version == 9 do
    [
      %MetadataResponse.Topic{
        error_code: 0,
        name: "foo",
        is_internal: false,
        topic_authorized_operations: 0xFF,
        partitions: partitions(api_version),
        tagged_fields: %{}
      }
    ]
  end

  def topics(api_version) do
    [
      %MetadataResponse.Topic{
        error_code: 0,
        name: "foo",
        topic_id: "i_am_fake_uuid_1",
        is_internal: false,
        topic_authorized_operations: 0xFF,
        partitions: partitions(api_version),
        tagged_fields: %{}
      }
    ]
  end

  def partitions(api_version) when api_version < 5 do
    [
      %MetadataResponse.Topic.Partition{
        error_code: 0,
        partition_index: 0,
        leader_id: 1,
        replica_nodes: [0, 1],
        isr_nodes: [0, 1]
      }
    ]
  end

  def partitions(api_version) when api_version < 7 do
    [
      %MetadataResponse.Topic.Partition{
        error_code: 0,
        partition_index: 0,
        leader_id: 1,
        replica_nodes: [0, 1],
        isr_nodes: [0, 1],
        offline_replicas: []
      }
    ]
  end

  def partitions(api_version) when api_version < 9 do
    [
      %MetadataResponse.Topic.Partition{
        error_code: 0,
        partition_index: 0,
        leader_id: 1,
        leader_epoch: 1,
        replica_nodes: [0, 1],
        isr_nodes: [0, 1],
        offline_replicas: []
      }
    ]
  end

  def partitions(_api_version) do
    [
      %MetadataResponse.Topic.Partition{
        error_code: 0,
        partition_index: 0,
        leader_id: 1,
        leader_epoch: 1,
        replica_nodes: [0, 1],
        isr_nodes: [0, 1],
        offline_replicas: [],
        tagged_fields: %{}
      }
    ]
  end
end
