defmodule Ockam.Services.Relay.StaticForwardingAPI do
  @moduledoc """
  API for static forwarding service

  See `Ockam.Services.StaticForwarding`
  """

  use Ockam.Services.API.Endpoint

  alias Ockam.API.Request
  alias Ockam.Services.API
  alias Ockam.Services.Relay.StaticForwarding, as: Base
  alias Ockam.Services.Relay.Types.CreateRelayRequest
  alias Ockam.Services.Relay.Types.Relay

  require Logger

  @impl true
  def init_endpoint(options) do
    with {:ok, state} <- Base.setup(options, %{address: "fwd_to"}) do
      {:ok, state,
       [
         {:identity, :get, "/", &list/2},
         {:identity, :post, "/", &create_relay/2},
         {:identity, :get, "/:addr", &get/2},
         {:identity, :delete, "/:addr", &delete/2}
       ]}
    end
  end

  @impl true
  def authorize(:identity, %Request{} = req, _bindings) do
    case Request.caller_identity_id(req) do
      {:ok, identifier} ->
        {true, %{identifier: identifier}}

      :error ->
        false
    end
  end

  def list(_req, %{
        auth_data: %{identifier: _identifier},
        state: _
      }) do
    Relay.encode_list(Base.list_running_relays())
  end

  def create_relay(%Request{body: body, from_route: from_route}, %{
        auth_data: %{identifier: identifier},
        state: state
      }) do
    with {:ok, %CreateRelayRequest{alias: alias, tags: tags}} <-
           CreateRelayRequest.decode_strict(body),
         :ok <- Base.check_authorization(identifier, alias),
         {:ok, worker_addr} <- Base.subscribe(alias, tags, from_route, identifier, false, state),
         {:ok, relay} <- wait_for_relay_worker(worker_addr) do
      Relay.encode(relay)
    else
      {:error, :not_authorized} ->
        {:error, :unauthorized}

      error ->
        Logger.error("Error creating relay: #{inspect(error)}")
        error
    end
  end

  # Workers perform initialization (including attaching metadata on the registry) _asynchronously_ after
  # returned from init(). This means we might get the addr _before_ any metadata is attached to it.
  # This wait for it to be available.
  # Note metadata is attached inmediately after init() returns, so in normal circustances there is no need
  # to wait, if the node is very busy we might need to wait for a short amount.
  # TODO: improve how workers starts up to avoid this.
  defp wait_for_relay_worker(worker_addr), do: wait_for_relay_worker(worker_addr, 5)

  defp wait_for_relay_worker(_worker_addr, 0), do: {:error, :timeout}

  defp wait_for_relay_worker(worker_addr, n) do
    case Base.relay_info(worker_addr) do
      {:ok, %Relay{created_at: c} = relay} when c != nil ->
        {:ok, relay}

      _other ->
        Process.sleep(50)
        wait_for_relay_worker(worker_addr, n - 1)
    end
  end

  @spec get(any, %{
          :auth_data => %{:identifier => any, optional(any) => any},
          :bindings => %{:addr => binary | Ockam.Address.t(), optional(any) => any},
          :state => any,
          optional(any) => any
        }) :: {:error, any} | {:ok, binary}
  def get(_req, %{
        bindings: %{addr: addr},
        auth_data: %{identifier: _identifier},
        state: _
      }) do
    case Base.relay_info(addr) do
      {:ok, relay} ->
        Relay.encode(relay)

      other ->
        Logger.warning(
          "Error attempting to retrieve relay information for addr #{inspect(addr)} : #{inspect(other)}"
        )

        {:error, 404}
    end
  end

  def delete(_req, %{
        bindings: %{addr: addr},
        auth_data: %{identifier: identifier},
        state: state
      }) do
    with :ok <- Base.check_authorization(identifier, addr),
         :ok <- Ockam.Node.stop(Base.forwarder_address(addr, state)) do
      {:ok, nil}
    else
      other ->
        Logger.warning(
          "Error attempting to delete relay information for addr #{inspect(addr)} (caller #{Ockam.Identity.Identifier.to_str(identifier)}) : #{inspect(other)}"
        )

        {:error, 401}
    end
  end
end
