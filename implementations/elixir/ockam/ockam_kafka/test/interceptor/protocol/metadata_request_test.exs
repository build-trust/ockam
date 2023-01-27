defmodule Ockam.Kafka.Interceptor.Protocol.Metadata.Request.Test do
  use ExUnit.Case

  alias Ockam.Kafka.Interceptor.Protocol.Formatter

  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Request, as: MetadataRequest
  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Request.Formatter, as: RequestFormatter
  alias Ockam.Kafka.Interceptor.Protocol.Metadata.Request.Parser, as: RequestParser

  alias Ockam.Kafka.Interceptor.Protocol.Parser

  alias Ockam.Kafka.Interceptor.Protocol.RequestHeader

  @api_metadata 3

  test "parse/format metadata request body" do
    ## Check with predefined binaries
    Enum.each(0..8, fn api_version ->
      request = metadata_request(api_version)

      expected = expected_request_data(api_version)

      assert {:ok, ^expected} = RequestFormatter.format(request)

      {:ok, ^request, <<>>} = RequestParser.parse(api_version, expected)
    end)

    ## Only check that format/parse match
    Enum.each(9..12, fn api_version ->
      request = metadata_request(api_version)

      {:ok, formatted} = RequestFormatter.format(request)

      {:ok, ^request, <<>>} = RequestParser.parse(api_version, formatted)
    end)
  end

  test "parse full metadata request" do
    Enum.each(0..8, fn api_version ->
      {:ok, header_version} = Parser.request_header_version(@api_metadata, api_version)
      request_header = request_header(header_version, api_version)

      {:ok, header_binary} = Formatter.format_request_header(request_header)

      request = metadata_request(api_version)

      expected = expected_request_data(api_version)
      size = byte_size(expected)

      message = header_binary <> expected

      assert {:ok, ^request_header, ^size, ^request} = Parser.parse_kafka_request(message)
    end)

    Enum.each(9..12, fn api_version ->
      {:ok, header_version} = Parser.request_header_version(@api_metadata, api_version)
      request_header = request_header(header_version, api_version)

      {:ok, header_binary} = Formatter.format_request_header(request_header)

      request = metadata_request(api_version)

      {:ok, formatted} = RequestFormatter.format(request)
      size = byte_size(formatted)

      message = header_binary <> formatted

      assert {:ok, ^request_header, ^size, ^request} = Parser.parse_kafka_request(message)
    end)
  end

  def topics(api_version) when api_version < 9 do
    ["foo", "bar"]
  end

  def topics(api_version) when api_version == 9 do
    [
      %MetadataRequest.Topic{
        name: "foo",
        tagged_fields: %{1 => "foo"}
      },
      %MetadataRequest.Topic{
        name: "bar",
        tagged_fields: %{1 => "bar"}
      }
    ]
  end

  def topics(_api_version) do
    [
      %MetadataRequest.Topic{
        name: "foo",
        topic_id: "i_am_fake_uuid_1",
        tagged_fields: %{1 => "foo"}
      },
      %MetadataRequest.Topic{
        name: "bar",
        topic_id: "i_am_fake_uuid_2",
        tagged_fields: %{1 => "bar"}
      }
    ]
  end

  def expected_request_data(api_version) when api_version < 4 do
    <<2::signed-big-integer-size(32), 3::signed-big-integer-size(16), "foo"::binary,
      3::signed-big-integer-size(16), "bar"::binary>>
  end

  def expected_request_data(api_version) when api_version < 8 do
    <<2::signed-big-integer-size(32), 3::signed-big-integer-size(16), "foo"::binary,
      3::signed-big-integer-size(16), "bar"::binary, 1>>
  end

  def expected_request_data(api_version) when api_version == 8 do
    <<2::signed-big-integer-size(32), 3::signed-big-integer-size(16), "foo"::binary,
      3::signed-big-integer-size(16), "bar"::binary, 1, 1, 1>>
  end

  def metadata_request(api_version) when api_version < 4 do
    %MetadataRequest{
      api_version: api_version,
      topics: topics(api_version)
    }
  end

  def metadata_request(api_version) when api_version < 8 do
    %MetadataRequest{
      api_version: api_version,
      topics: topics(api_version),
      allow_auto_topic_creation: true
    }
  end

  def metadata_request(api_version) when api_version == 8 do
    %MetadataRequest{
      api_version: api_version,
      topics: topics(api_version),
      allow_auto_topic_creation: true,
      include_cluster_authorized_operations: true,
      include_topic_authorized_operations: true
    }
  end

  def metadata_request(api_version) when api_version < 11 do
    %MetadataRequest{
      api_version: api_version,
      topics: topics(api_version),
      allow_auto_topic_creation: true,
      include_cluster_authorized_operations: true,
      include_topic_authorized_operations: true,
      tagged_fields: %{1 => "baz"}
    }
  end

  def metadata_request(api_version) do
    %MetadataRequest{
      api_version: api_version,
      topics: topics(api_version),
      allow_auto_topic_creation: true,
      include_topic_authorized_operations: true,
      tagged_fields: %{1 => "baz"}
    }
  end

  def request_header(1, api_version) do
    %RequestHeader{
      header_version: 1,
      api_key: @api_metadata,
      api_version: api_version,
      correlation_id: 10,
      client_id: "foo"
    }
  end

  def request_header(2, api_version) do
    %RequestHeader{
      header_version: 2,
      api_key: @api_metadata,
      api_version: api_version,
      correlation_id: 10,
      client_id: "foo",
      tagged_fields: %{2 => "foo", 3 => "bar"}
    }
  end
end
