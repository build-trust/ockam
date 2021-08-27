defmodule Ockam.Protocol.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Protocol
  alias Ockam.Protocol
  alias Ockam.Protocol.Tests.ExampleProtocol

  describe "Base message" do
    test "can be encoded and decoded" do
      name = "my_protocol_name"
      data = "some arbitrary data"
      base_message = %{protocol: name, data: data}

      encoded = Protocol.base_encode(name, data)

      assert is_binary(encoded)

      {:ok, decoded} = Protocol.base_decode(encoded)

      assert ^decoded = base_message
    end
  end

  describe "Protocol message" do
    test "can be encoded and decoded" do
      struct_request = %{string_field: "I am string", int_field: 10}
      data_request = "I am data request"
      data_response = "I am data response"

      struct_request_encoded =
        Protocol.encode(ExampleProtocol, :request, {:structure, struct_request})

      assert is_binary(struct_request_encoded)

      assert {:ok, {:structure, ^struct_request}} =
               Protocol.decode(ExampleProtocol, :request, struct_request_encoded)

      data_request_encoded = Protocol.encode(ExampleProtocol, :request, {:data, data_request})

      assert {:ok, {:data, ^data_request}} =
               Protocol.decode(ExampleProtocol, :request, data_request_encoded)

      data_response_encoded = Protocol.encode(ExampleProtocol, :response, data_response)

      assert {:ok, ^data_response} =
               Protocol.decode(ExampleProtocol, :response, data_response_encoded)

      assert catch_error(Protocol.encode(ExampleProtocol, :response, struct_request)) ==
               :cannot_encode
    end
  end

  describe "Protocol message wrapped in the base message" do
    test "can be encoded and decoded" do
      struct_request = %{string_field: "I am string", int_field: 10}

      struct_request_encoded =
        Protocol.encode_payload(ExampleProtocol, :request, {:structure, struct_request})

      assert is_binary(struct_request_encoded)

      assert {:ok, %{protocol: "example_protocol"}} = Protocol.base_decode(struct_request_encoded)

      assert {:ok, {:structure, ^struct_request}} =
               Protocol.decode_payload(ExampleProtocol, :request, struct_request_encoded)
    end
  end
end
