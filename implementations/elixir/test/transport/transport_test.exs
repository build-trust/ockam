defmodule Ockam.Transport.Test do
  use ExUnit.Case, async: false
  require Logger

  alias Ockam.Channel
  alias Ockam.Transport.Address
  alias Ockam.Transport.Socket
  alias Ockam.Vault
  alias Ockam.Vault.KeyPair
  alias Ockam.Vault.SecretAttributes

  setup context do
    {:ok, vault} = Vault.new()

    if transport = context[:transport] do
      name = Map.fetch!(context, :transport_name)
      meta = [name: name]
      config = Map.get(context, :transport_config, [])
      pid = start_supervised!({transport, [meta, config]})
      {:ok, [vault: vault, pid: pid, config: config]}
    else
      {:ok, [vault: vault]}
    end
  end

  @tag transport: Ockam.Transport.TCP
  @tag transport_name: :tcp_4001
  @tag transport_config: [listen_address: "0.0.0.0", listen_port: 4001]
  test "tcp transport", %{vault: vault, config: config} do
    {:ok, addr} = Address.new(:inet, :loopback, config[:listen_port])
    socket = Socket.new(:client, addr)

    attrs = SecretAttributes.x25519(:ephemeral)
    s = KeyPair.new(vault, attrs)
    e = KeyPair.new(vault, attrs)
    rs = KeyPair.new(vault, attrs)
    re = KeyPair.new(vault, attrs)

    handshake_opts = %{protocol: "Noise_XX_25519_AESGCM_SHA256", s: s, e: e, rs: rs, re: re}
    assert {:ok, handshake} = Channel.handshake(vault, :initiator, handshake_opts)
    assert {:ok, transport} = Socket.open(socket)

    assert {:ok, _chan, transport} =
             Channel.negotiate_secure_channel(handshake, transport, %{timeout: 10_000})

    assert {:ok, _} = Socket.close(transport)
  end
end
