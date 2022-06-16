defmodule Ockam.SecureChannel.Listener do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.SecureChannel.Channel
  alias Ockam.SecureChannel.InitHandshake

  require Logger

  @impl true
  def address_prefix(_options), do: "SC_"

  @doc false
  @impl true
  def setup(options, state) do
    with {:ok, vault} <- Keyword.fetch(options, :vault),
         {:ok, identity_keypair} <- Keyword.fetch(options, :identity_keypair) do
      state = Map.put(state, :vault, vault)
      state = Map.put(state, :identity_keypair, identity_keypair)
      {:ok, state}
    else
      :error -> {:error, {:required_options_missing, [:vault, :identity_keypair], options}}
    end
  end

  @doc false
  @impl true
  def handle_message(message, state) do
    create_channel(message, state)
  end

  defp create_channel(message, state) do
    payload = Message.payload(message)

    ## TODO: is there any other options possible?
    base_channel_options =
      [role: :responder]
      |> Keyword.put(:vault, state.vault)
      |> Keyword.put(:identity_keypair, state.identity_keypair)

    with {:ok, init_handshake} <- InitHandshake.decode(payload),
         {:ok, responder_init_message} <- make_responder_init_message(message, init_handshake),
         channel_options <-
           Keyword.put(base_channel_options, :initiating_message, responder_init_message),
         {:ok, _address} <- Channel.create(channel_options ++ [restart_type: :temporary]) do
      {:ok, state}
    end
  end

  defp make_responder_init_message(message, init_handshake) do
    return_route = Message.return_route(message)

    {:ok,
     %Ockam.Message{
       onward_route: [],
       return_route: return_route,
       payload: Map.get(init_handshake, :handshake)
     }}
  end
end
