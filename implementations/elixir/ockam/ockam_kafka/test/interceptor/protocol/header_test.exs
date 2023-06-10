defmodule Ockam.Kafka.Interceptor.Protocol.HeaderTest do
  use ExUnit.Case

  alias Ockam.Kafka.Interceptor.Protocol.Formatter
  alias Ockam.Kafka.Interceptor.Protocol.Parser

  alias Ockam.Kafka.Interceptor.Protocol.RequestHeader
  alias Ockam.Kafka.Interceptor.Protocol.ResponseHeader

  @api_metadata 3

  test "parse/format request header" do
    v0 = %RequestHeader{
      header_version: 0,
      api_key: @api_metadata,
      api_version: 1,
      correlation_id: 10
    }

    {:ok, v0_formatted} = Formatter.format_request_header(v0)

    <<3::signed-big-integer-size(16), 1::signed-big-integer-size(16),
      10::signed-big-integer-size(32)>> = v0_formatted

    {:ok, ^v0, <<>>} = Parser.parse_request_header(0, v0_formatted)

    v1 = %RequestHeader{
      header_version: 1,
      api_key: @api_metadata,
      api_version: 1,
      correlation_id: 10,
      client_id: "foo"
    }

    {:ok, v1_formatted} = Formatter.format_request_header(v1)

    <<3::signed-big-integer-size(16), 1::signed-big-integer-size(16),
      10::signed-big-integer-size(32), _client_id::binary>> = v1_formatted

    {:ok, ^v1, <<>>} = Parser.parse_request_header(1, v1_formatted)

    v2 = %RequestHeader{
      header_version: 2,
      api_key: @api_metadata,
      api_version: 1,
      correlation_id: 10,
      client_id: "foo",
      tagged_fields: %{1 => "bar"}
    }

    {:ok, v2_formatted} = Formatter.format_request_header(v2)

    <<3::signed-big-integer-size(16), 1::signed-big-integer-size(16),
      10::signed-big-integer-size(32), _rest::binary>> = v2_formatted

    {:ok, ^v2, <<>>} = Parser.parse_request_header(2, v2_formatted)
  end

  test "parse/format response header" do
    v0 = %ResponseHeader{header_version: 0, correlation_id: 10}
    request_header = %RequestHeader{correlation_id: 10}

    {:ok, v0_formatted} = Formatter.format_response_header(v0)

    <<10::signed-big-integer-size(32)>> = v0_formatted

    {:ok, ^v0, <<>>} = Parser.parse_response_header(0, request_header, v0_formatted)

    v1 = %ResponseHeader{header_version: 1, correlation_id: 10, tagged_fields: %{1 => "foo"}}

    {:ok, v1_formatted} = Formatter.format_response_header(v1)
    <<10::signed-big-integer-size(32), _fields::binary>> = v1_formatted

    {:ok, ^v1, <<>>} = Parser.parse_response_header(1, request_header, v1_formatted)
  end
end
