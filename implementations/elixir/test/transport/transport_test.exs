defmodule Ockam.Transport.Test do
  use ExUnit.Case, async: true
  require Logger

  alias Ockam.Channel
  alias Ockam.Transport.Address
  alias Ockam.Transport.Socket
  alias Ockam.Vault.KeyPair

  setup context do
    if transport = context[:transport] do
      name = Map.fetch!(context, :transport_name)
      meta = [name: name]
      config = Map.get(context, :transport_config, [])
      pid = start_supervised!({transport, [meta, config]})
      {:ok, [pid: pid, config: config]}
    else
      {:ok, []}
    end
  end

  @tag transport: Ockam.Transport.TCP
  @tag transport_name: :tcp_4000
  @tag transport_config: [listen_address: "0.0.0.0", listen_port: 4000]
  test "tcp transport", %{config: config} do
    {:ok, addr} = Address.new(:inet, :loopback, config[:listen_port])
    socket = Socket.new(:client, addr)

    s = KeyPair.new(:x25519)
    e = KeyPair.new(:x25519)
    rs = KeyPair.new(:x25519)
    re = KeyPair.new(:x25519)

    handshake_opts = %{protocol: "Noise_XX_25519_AESGCM_SHA256", s: s, e: e, rs: rs, re: re}
    assert {:ok, handshake} = Channel.handshake(:initiator, handshake_opts)
    assert {:ok, transport} = Socket.open(socket)
    assert {:ok, _chan, transport} = Channel.negotiate_secure_channel(handshake, transport)
    assert {:ok, _} = Socket.close(transport)
  end
end
