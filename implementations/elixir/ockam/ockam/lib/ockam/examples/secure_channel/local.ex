defmodule Ockam.Examples.SecureChannel.Local do
  @moduledoc """
  Local node secure channel example

  run/0 - run the example. Creates a secure channel and sends ping-pong messages through it
  """

  ## Ignore no local return for secure channel
  @dialyzer [:no_return, {:nowarn_function, wait: 1}]

  alias Ockam.SecureChannel
  alias Ockam.Vault
  alias Ockam.Vault.Software, as: SoftwareVault

  alias Ockam.Example.Stream.Ping
  alias Ockam.Example.Stream.Pong

  require Logger

  def run() do
    responder()

    initiator()
  end

  def responder() do
    {:ok, "pong"} = Pong.create(address: "pong")
    create_secure_channel_listener()
  end

  def initiator() do
    {:ok, "ping"} = Ping.create(address: "ping")

    {:ok, channel} = create_secure_channel(["SC_listener"])

    send_message([channel, "pong"], ["ping"], "0")
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

  def send_message(onward_route, return_route, payload) do
    msg = %{
      onward_route: onward_route,
      return_route: return_route,
      payload: payload
    }

    Ockam.Router.route(msg)
  end
end
