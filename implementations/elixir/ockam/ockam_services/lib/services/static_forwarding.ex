defmodule Ockam.Services.StaticForwarding do
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

  alias __MODULE__.Forwarder
  alias Ockam.Message

  require Logger

  @impl true
  def setup(options, state) do
    prefix = Keyword.get(options, :prefix, state.address)
    {:ok, Map.put(state, :prefix, prefix)}
  end

  @impl true
  def handle_message(message, state) do
    payload = Message.payload(message)

    case :bare.decode(payload, :string) do
      {:ok, alias_str, ""} ->
        return_route = Message.return_route(message)
        subscribe(alias_str, return_route, state)

      err ->
        Logger.error("Invalid message format: #{inspect(payload)}, reason #{inspect(err)}")
    end
  end

  def subscribe(alias_str, route, state) do
    with {:ok, worker} <- ensure_alias_worker(alias_str, state) do
      ## NOTE: Non-ockam message routing here
      Forwarder.update_route(worker, route)
      {:ok, state}
    end
  end

  def ensure_alias_worker(alias_str, state) do
    forwarder_address = forwarder_address(alias_str, state)

    case Ockam.Node.whereis(forwarder_address) do
      nil -> Forwarder.create(alias: alias_str, address: forwarder_address)
      _pid -> {:ok, forwarder_address}
    end
  end

  def forwarder_address(alias_str, state) do
    Map.get(state, :prefix, "") <> "_" <> alias_str
  end
end

defmodule Ockam.Services.StaticForwarding.Forwarder do
  @moduledoc """
  Topic subscription for pub_sub service

  Forwards all messages to all subscribed routes

  Subscribe API is internal, it adds a route to the subscribers set
  """
  use Ockam.Worker

  alias Ockam.Message

  def update_route(worker, route, options \\ []) do
    ## TODO: reply to the subscriber?
    Ockam.Worker.call(worker, {:update_route, route, options})
  end

  @impl true
  def setup(options, state) do
    alias_str = Keyword.get(options, :alias)
    {:ok, Map.merge(state, %{alias: alias_str, route: []})}
  end

  @impl true
  def handle_call({:update_route, route, options}, _from, %{alias: alias_str} = state) do
    state = Map.put(state, :route, route)

    case Keyword.get(options, :notify, true) do
      true ->
        Ockam.Router.route(%{
          onward_route: route,
          return_route: [state.address],
          payload: :bare.encode("#{alias_str}", :string)
        })

      false ->
        :ok
    end

    {:reply, :ok, state}
  end

  @impl true
  def handle_message(message, state) do
    [_me | onward_route] = Message.onward_route(message)

    route = Map.get(state, :route, [])

    Ockam.Router.route(Message.forward(message, route ++ onward_route))

    {:ok, state}
  end
end
