defmodule Ockam.Examples.SecureChannel.Local do
  @moduledoc """
  Local node secure channel example

  run/0 - run the example. Creates a secure channel and sends a message

  send_and_wait/0 - send more messages through the channel
  """

  ## Ignore no local return for secure channel
  @dialyzer [:no_return, {:nowarn_function, wait: 1}]

  alias Ockam.SecureChannel
  alias Ockam.Vault
  alias Ockam.Vault.Software, as: SoftwareVault

  alias Ockam.Examples.Echoer

  require Logger

  def run() do
    responder()

    initiator()
  end

  def responder() do
    {:ok, "echoer"} = Echoer.create(address: "echoer")
    create_secure_channel_listener()
  end

  def initiator() do
    {:ok, channel} = create_secure_channel(["SC_listener"])

    ## Register this process to receive messages
    my_address = "example_run"
    Ockam.Node.register_address(my_address)

    send_and_wait(channel, "Hello secure channel!", my_address)

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
