defmodule Ockam.Test.Support.Influx.Client do
  import ExUnit.Assertions

  alias Ockam.Channel
  alias Ockam.Vault.KeyPair
  alias Ockam.Vault.SecretAttributes
  alias Ockam.Transport.Address
  alias Ockam.Transport.Socket
  alias Ockam.Services.Influx
  alias Ockam.Router.Protocol.Encoding
  alias Ockam.Router.Protocol.Encoder
  alias Ockam.Router.Protocol.Endpoint
  alias Ockam.Router.Protocol.Message
  alias Ockam.Router.Protocol.Message.Envelope

  defstruct [:vault, :service, :address, :chan, :transport, :timeout]

  def new(vault, config) do
    with {:ok, addr} <- Address.new(:inet, :loopback, config[:listen_port]) do
      {:ok,
       %__MODULE__{
         vault: vault,
         service: config[:service],
         address: addr,
         timeout: config[:timeout] || 10_000
       }}
    end
  end

  def connect(%__MODULE__{vault: vault, address: addr} = state) do
    attrs = SecretAttributes.x25519(:ephemeral)
    s = KeyPair.new(vault, attrs)
    e = KeyPair.new(vault, attrs)
    rs = KeyPair.new(vault, attrs)
    re = KeyPair.new(vault, attrs)

    handshake_opts = %{protocol: "Noise_XX_25519_AESGCM_SHA256", s: s, e: e, rs: rs, re: re}
    assert {:ok, handshake} = Channel.handshake(vault, :initiator, handshake_opts)
    socket = Socket.new(:client, addr)
    assert {:ok, transport} = Socket.open(socket)

    assert {:ok, chan, new_transport} =
             Channel.negotiate_secure_channel(handshake, transport, %{timeout: state.timeout})

    # Await ping
    assert {:ok, encrypted, new_transport} = Socket.recv(new_transport, timeout: state.timeout)
    assert {:ok, new_chan, encoded} = Channel.decrypt(chan, encrypted)
    assert {:ok, %Envelope{body: %Message.Ping{}}, _} = Encoding.decode(encoded)

    # Reply with pong
    assert {:ok, encoded} = Encoding.encode(%Message.Pong{})
    assert {:ok, new_chan, encrypted} = Channel.encrypt(new_chan, encoded)
    assert {:ok, new_transport} = Socket.send(new_transport, encrypted)

    # Initiate influx connection
    send_to = Endpoint.new(%Endpoint.Local{data: to_string(state.service)})
    headers = %{:send_to => send_to}

    assert {:ok, encoded} = Encoding.encode(%Envelope{headers: headers, body: %Message.Connect{}})

    assert {:ok, new_chan, encrypted} = Channel.encrypt(new_chan, encoded)
    assert {:ok, new_transport} = Socket.send(new_transport, encrypted)

    await_ack(%__MODULE__{state | chan: new_chan, transport: new_transport})
  end

  def write(%__MODULE__{chan: chan, transport: transport} = state, measurement, tags, fields) do
    assert {:ok, write_encoded} =
             Encoder.encode(
               %Influx.Message.Write{
                 measurement: measurement,
                 tags: tags,
                 fields: fields
               },
               %{}
             )

    assert {:ok, encoded} =
             Encoding.encode(%Message.Send{data: IO.iodata_to_binary(write_encoded)})

    assert {:ok, new_chan, encrypted} = Channel.encrypt(chan, encoded)
    assert {:ok, new_transport} = Socket.send(transport, encrypted)

    await_ack(%__MODULE__{state | chan: new_chan, transport: new_transport})
  end

  def query(%__MODULE__{chan: chan, transport: transport} = state, query_text) do
    assert {:ok, query_encoded} = Encoder.encode(%Influx.Message.Query{text: query_text}, %{})

    assert {:ok, encoded} =
             Encoding.encode(%Message.Request{data: IO.iodata_to_binary(query_encoded)})

    assert {:ok, new_chan, encrypted} = Channel.encrypt(chan, encoded)
    assert {:ok, new_transport} = Socket.send(transport, encrypted)

    await_results(%__MODULE__{state | chan: new_chan, transport: new_transport})
  end

  def transport(%__MODULE__{transport: transport}), do: transport

  defp await_ack(%__MODULE__{chan: chan, transport: transport} = state) do
    assert {:ok, encrypted, new_transport} = Socket.recv(transport, timeout: state.timeout)
    assert {:ok, new_chan, encoded} = Channel.decrypt(chan, encrypted)
    assert {:ok, %Envelope{body: body}, _} = Encoding.decode(encoded)
    new_state = %__MODULE__{state | chan: new_chan, transport: new_transport}

    case body do
      %Message.Ack{} ->
        {:ok, new_state}

      %Message.Error{description: desc} ->
        {:error, new_state, desc}

      other ->
        {:error, new_state, {:expected_ack_or_error, other}}
    end
  end

  defp await_results(%__MODULE__{chan: chan, transport: transport} = state) do
    assert {:ok, encrypted, new_transport} = Socket.recv(transport, timeout: state.timeout)
    assert {:ok, new_chan, encoded} = Channel.decrypt(chan, encrypted)
    assert {:ok, %Envelope{body: body}, _} = Encoding.decode(encoded)

    new_state = %__MODULE__{state | chan: new_chan, transport: new_transport}

    case body do
      %Message.Payload{data: results} ->
        {:ok, new_state, Jason.decode!(results, keys: :atoms)}

      %Message.Error{description: desc} ->
        {:error, new_state, desc}

      other ->
        {:error, new_state, {:expected_payload_or_error, other}}
    end
  end
end
