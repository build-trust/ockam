defmodule Ockam.Example.Stream.Transport do
  @moduledoc false

  alias Ockam.Example.Stream.Ping
  alias Ockam.Example.Stream.Pong

  alias Ockam.SecureChannel
  alias Ockam.Vault
  alias Ockam.Vault.Software, as: SoftwareVault

  alias Ockam.Stream.Transport.Address, as: StreamAddress

  @hub_tcp %Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: 4000}

  def init_ping() do
    ensure_tcp(3000)
    Ping.create(address: "ping")
    # subscribe("ping_topic", [])
  end

  def init_pong() do
    ensure_tcp(5000)
    Pong.create(address: "pong")
    subscribe("pong_topic", [])
  end

  def subscribe(stream, route) do
    subscribe_msg = %{
      onward_route: [
        @hub_tcp,
        "stream_transport_subscribe"
      ],
      return_route: route,
      payload: Ockam.Protocol.encode_payload(Ockam.Protocol.Binary, :request, stream)
    }

    Ockam.Router.route(subscribe_msg)
  end

  def run() do
    init_ping()

    send_message()
  end

  def send_message() do
    msg = %{
      onward_route: [
        @hub_tcp,
        %StreamAddress{onward_stream: "pong_topic", return_stream: "ping_topic"},
        "pong"
      ],
      return_route: ["ping"],
      payload: "0"
    }

    Ockam.Router.route(msg)
  end

  def secure_channel_listener() do
    secret_attributes = %{type: :curve25519, persistence: :ephemeral, length: 32}

    {:ok, responder_vault} = SoftwareVault.init()
    {:ok, responder_identity} = Vault.secret_generate(responder_vault, secret_attributes)

    SecureChannel.create_listener(
      vault: responder_vault,
      identity_keypair: responder_identity,
      address: "secure_channel_listener"
    )
  end

  def secure_channel_tcp(port) do
    tcp_address = %Ockam.Transport.TCPAddress{ip: {127, 0, 0, 1}, port: port}
    secret_attributes = %{type: :curve25519, persistence: :ephemeral, length: 32}

    {:ok, initiator_vault} = SoftwareVault.init()
    {:ok, initiator_identity} = Vault.secret_generate(initiator_vault, secret_attributes)

    {:ok, _c} =
      SecureChannel.create(
        route: [tcp_address, "secure_channel_listener"],
        vault: initiator_vault,
        identity_keypair: initiator_identity
      )
  end

  def ensure_tcp(port) do
    Ockam.Transport.TCP.create_listener(port: port, route_outgoing: true)
  end
end
