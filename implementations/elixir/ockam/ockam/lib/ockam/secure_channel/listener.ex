defmodule Ockam.SecureChannel.Listener do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.SecureChannel.Channel

  @impl true
  def address_prefix(_options), do: "SC_"

  @doc false
  @impl true
  def setup(options, state) do
    with {:ok, vault} <- get_from_options(:vault, options),
         {:ok, identity_keypair} <- get_from_options(:identity_keypair, options) do
      state = Map.put(state, :vault, vault)
      state = Map.put(state, :identity_keypair, identity_keypair)
      {:ok, state}
    end
  end

  @doc false
  @impl true
  def handle_message(message, state) do
    create_channel(message, state)
  end

  defp create_channel(message, state) do
    channel_options =
      [role: :responder]
      |> Keyword.put(:vault, state.vault)
      |> Keyword.put(:identity_keypair, state.identity_keypair)

    with {:ok, channel_options} <- update_routes(message, channel_options),
         {:ok, _address} <- Channel.create(channel_options) do
      {:ok, state}
    end
  end

  defp update_routes(message, channel_options) do
    onward_route = Message.onward_route(message)
    return_route = Message.return_route(message)
    payload = Message.payload(message)

    {_address, onward_route} = List.pop_at(onward_route, length(onward_route) - 1)

    message = %Ockam.Message{
      onward_route: onward_route,
      return_route: return_route,
      payload: payload
    }

    channel_options = Keyword.put(channel_options, :initiating_message, message)
    {:ok, channel_options}
  end
end
