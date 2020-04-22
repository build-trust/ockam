defmodule Ockam.Router.Protocol.Encoding.Test do
  use ExUnit.Case, async: true
  require Logger

  alias Ockam.Router.Protocol.Message
  alias Ockam.Router.Protocol.Message.Envelope
  alias Ockam.Router.Protocol.Encoding
  alias Ockam.Router.Protocol.Endpoint
  alias Ockam.Transport.Address

  test "ping" do
    ping = %Message.Ping{}
    opts = %{}

    assert {:ok, encoded} = Encoding.encode(ping, opts)
    assert {:ok, %Envelope{body: ^ping}, <<>>} = Encoding.decode(encoded, opts)
  end

  test "pong" do
    pong = %Message.Pong{}
    opts = %{}

    assert {:ok, encoded} = Encoding.encode(pong, opts)
    assert {:ok, %Envelope{body: ^pong}, <<>>} = Encoding.decode(encoded, opts)
  end

  test "payloads" do
    payload = %Message.Payload{data: "hello"}
    opts = %{}

    assert {:ok, encoded} = Encoding.encode(payload, opts)
    assert {:ok, %Envelope{body: ^payload}, <<>>} = Encoding.decode(encoded, opts)

    tag = String.duplicate("t", 16)
    encrypted_payload = %Message.EncryptedPayload{data: "hello", tag: tag}

    assert {:ok, encoded} = Encoding.encode(encrypted_payload, opts)
    assert {:ok, %Envelope{body: ^encrypted_payload}, <<>>} = Encoding.decode(encoded, opts)
  end

  test "connect" do
    connect = %Message.Connect{
      options: [
        %Message.Connect.Option{name: "foo", value: "bar"},
        %Message.Connect.Option{name: "baz", value: "qux"}
      ]
    }

    opts = %{}

    assert {:ok, encoded} = Encoding.encode(connect, opts)
    assert {:ok, %Envelope{body: ^connect}, <<>>} = Encoding.decode(encoded, opts)
  end

  test "payload with headers" do
    headers = %{
      send_to: Endpoint.new_ipv4(:tcp, Address.new!(:inet, :any, 8080)),
      reply_to: Endpoint.new_ipv6(:tcp, Address.new!(:inet, "::1", 8081))
    }

    payload = %Message.Payload{data: "hello"}
    message = %Envelope{headers: headers, body: payload}
    opts = %{}

    assert {:ok, encoded} = Encoding.encode(message, opts)

    assert {:ok, %Envelope{headers: ^headers, body: ^payload}, <<>>} =
             Encoding.decode(encoded, opts)
  end
end
