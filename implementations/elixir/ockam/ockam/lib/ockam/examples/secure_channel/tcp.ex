defmodule Ockam.Examples.SecureChannel.TCP do
  @moduledoc """
  Local node secure channel example

  responder() - creates a secure channel listener and pong worker
  responder(port) - creates a responder listening on port default is 5000
  initiator() - creates a secure channel to the responder, ping worker and starts the ping-pong process
  initiator(host, port) - creates a secure channel to a responder on host:port

  send_and_wait/0 - send more messages through existing channel

  Responder should be running for initiator to work
  """

  ## Ignore no local return for secure channel
  @dialyzer [:no_return, {:nowarn_function, wait: 1}]

  alias Ockam.SecureChannel
  alias Ockam.Vault
  alias Ockam.Vault.Software, as: SoftwareVault

  alias Ockam.Examples.Echoer

  alias Ockam.Transport.TCP
  alias Ockam.Transport.TCPAddress

  require Logger

  @default_host "localhost"
  @default_port 5000

  def responder(port \\ @default_port) do
    TCP.start(listen: [port: port])

    {:ok, "echoer"} = Echoer.create(address: "echoer")
    create_secure_channel_listener()
  end

  def initiator(host \\ @default_host, port \\ @default_port) do
    TCP.start()

    tcp_address = TCPAddress.new(host, port)
    {:ok, channel} = create_secure_channel([tcp_address, "SC_listener"])

    ## Register this process to receive messages
    my_address = "example_run"
    Ockam.Node.register_address(my_address)

    send_and_wait(channel, "Hello secure channel over TCP!", my_address)

    {:ok, channel}
  end

  def send_and_wait(channel, message, return_address \\ "example_run") do
    Ockam.Router.route(%{
      onward_route: [channel, "echoer"],
      return_route: [return_address],
      payload: message
    })

    receive do
      %{
        onward_route: [^return_address],
        return_route: _return_route,
        payload: ^message
      } = reply ->
        Logger.info("Received message: #{inspect(reply)}")
        :ok
    end
  end

  defp create_secure_channel_listener() do
    {:ok, vault} = SoftwareVault.init()
    {:ok, identity} = Vault.secret_generate(vault, type: :curve25519)

    SecureChannel.create_listener(
      vault: vault,
      identity_keypair: identity,
      address: "SC_listener"
    )
  end

  defp create_secure_channel(route_to_listener) do
    {:ok, vault} = SoftwareVault.init()
    {:ok, identity} = Vault.secret_generate(vault, type: :curve25519)

    {:ok, c} =
      SecureChannel.create(route: route_to_listener, vault: vault, identity_keypair: identity)

    wait(fn -> SecureChannel.established?(c) end)
    {:ok, c}
  end

  defp wait(fun) do
    case fun.() do
      true ->
        :ok

      false ->
        :timer.sleep(100)
        wait(fun)
    end
  end
end
