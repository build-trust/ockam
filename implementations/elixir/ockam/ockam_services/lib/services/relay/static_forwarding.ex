defmodule Ockam.Services.Relay.StaticForwarding do
  @moduledoc """
  Static forwarding service

  Subscribes workers (by return route) to a string forwarding alias

  Forwarding alias is parsed from the payload as a BARE `string` type

  New subscriptions update the forwarding route in the same forwarding alias

  Forwarder address is created from prefix and alias as <prefix>_<alias>
  e.g. if prefix is `forward_to_` and alias is `my_alias`, forwarder address will be: `forward_to_my_alias`

  Messages sent to the forwarder address will be forwarded to the forwarding route

  Options:

  `prefix` - address prefix
  """
  use Ockam.Worker

  alias Ockam.Services.Relay.Types.CreateRelayRequest
  alias Ockam.Services.Relay.Types.Relay
  alias Ockam.Services.Relay.Worker, as: Forwarder

  alias Ockam.Message

  require Logger

  @spec list_running_relays() :: [{Ockam.Address.t(), map()}]
  def list_running_relays() do
    Ockam.Node.Registry.select_by_attribute(:service, :relay)
    |> Enum.map(&Relay.from_registry_attributes/1)
  end

  @spec relay_info(addr :: Ockam.Address.t()) :: {:ok, Relay.t()} | :error
  def relay_info(addr) do
    with {:ok, meta} <- Ockam.Node.Registry.lookup_meta(addr) do
      {:ok, Relay.from_registry_attributes({addr, meta.attributes})}
    end
  end

  @impl true
  def setup(options, state) do
    prefix = Keyword.get(options, :prefix, state.address)

    forwarder_options = Keyword.get(options, :forwarder_options, [])

    {:ok,
     Map.merge(state, %{
       prefix: prefix,
       forwarder_options: forwarder_options
     })}
  end

  @impl true
  def handle_message(message, state) do
    payload = Message.payload(message)

    case parse_create_relay_req(payload) do
      {:ok, req} ->
        return_route = Message.return_route(message)
        target_identifier = Message.local_metadata_value(message, :identity_id)

        case subscribe(req.alias, req.tags, return_route, target_identifier, true, state) do
          {:ok, _addr} ->
            :ok

          {:error, reason} ->
            Logger.warning(
              "Error creating/updating relay (alias #{req.alias}) (caller identifier #{inspect(target_identifier)})  #{inspect(reason)}"
            )
        end

        {:ok, state}

      {:error, reason} ->
        Logger.error("Invalid relay create msg: #{inspect(payload)}, reason #{inspect(reason)}")
        {:ok, state}
    end
  end

  def parse_create_relay_req(data) do
    case :bare.decode(data, :string) do
      {:ok, alias_str, ""} ->
        {:ok, %CreateRelayRequest{alias: alias_str, tags: %{}}}

      _err ->
        CreateRelayRequest.decode_strict(data)
    end
  end

  def subscribe(alias_str, tags, route, target_identifier, notify, state) do
    forwarder_address = forwarder_address(alias_str, state)
    forwarder_options = Map.fetch!(state, :forwarder_options)

    case Ockam.Node.whereis(forwarder_address) do
      nil ->
        Forwarder.create(
          Keyword.merge(forwarder_options,
            address: forwarder_address,
            relay_options: [
              alias: alias_str,
              route: route,
              tags: tags,
              notify: notify,
              target_identifier: target_identifier
            ]
          )
        )

      _pid ->
        with :ok <-
               Forwarder.update_route(forwarder_address, route, target_identifier, tags, notify) do
          {:ok, forwarder_address}
        end
    end
  end

  def forwarder_address(alias_str, state) do
    Map.get(state, :prefix, "") <> "_" <> alias_str
  end
end
