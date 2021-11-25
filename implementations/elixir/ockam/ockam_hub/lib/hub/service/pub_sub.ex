defmodule Ockam.Hub.Service.PubSub do
  @moduledoc """
  PubSub service

  Subscribes workers (by return route) to a string topic

  Each topic can have multiple subscrtiptions
  Each subscription has a unique name and topic

  Name and topic are parsed from the payload as a BARE `string` type
  with the following format "name:topic".
  Name cannot contain `:` symbol

  New subscriptions with the same name replace previous ones.

  Topic address is created from topic prefix and topic as <prefix>_<topic>
  e.g. if prefix is `pub_sub_t` and topic is `my_topic`, topic address will be: `pub_sub_t_my_topic`

  Messages sent to the topic address will be forwarded to all subscribers routes

  Options:

  `prefix` - topic address prefix
  """
  use Ockam.Worker

  alias __MODULE__.Topic
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

    with {:ok, name_topic, ""} <- :bare.decode(payload, :string),
         [name, topic] <- String.split(name_topic, ":") do
      return_route = Message.return_route(message)
      subscribe(name, topic, return_route, state)
    else
      err ->
        Logger.error("Invalid message format: #{inspect(payload)}, reason #{inspect(err)}")
    end
  end

  def subscribe(name, topic, route, state) do
    with {:ok, worker} <- ensure_topic_worker(topic, state) do
      ## NOTE: Non-ockam message routing here
      Topic.subscribe(worker, name, route)
      {:ok, state}
    end
  end

  def ensure_topic_worker(topic, state) do
    topic_address = topic_address(topic, state)

    case Ockam.Node.whereis(topic_address) do
      nil -> Topic.create(topic: topic, address: topic_address)
      _pid -> {:ok, topic_address}
    end
  end

  def topic_address(topic, state) do
    Map.get(state, :prefix, "") <> "_" <> topic
  end
end

defmodule Ockam.Hub.Service.PubSub.Topic do
  @moduledoc """
  Topic subscription for pub_sub service

  Forwards all messages to all subscribed routes

  Subscribe API is internal, it adds a route to the subscribers set
  """
  use Ockam.Worker

  alias Ockam.Message

  def subscribe(worker, name, route) do
    ## TODO: reply to the subscriber?
    Ockam.Worker.call(worker, {:subscribe, name, route})
  end

  @impl true
  def setup(options, state) do
    topic = Keyword.get(options, :topic)
    {:ok, Map.merge(state, %{topic: topic, routes: %{}})}
  end

  @impl true
  def handle_call({:subscribe, name, route}, _from, %{topic: topic} = state) do
    state =
      Map.update(state, :routes, %{name => route}, fn routes -> Map.put(routes, name, route) end)

    Ockam.Router.route(%{
      onward_route: route,
      return_route: [state.address],
      payload: :bare.encode("#{name}:#{topic}", :string)
    })

    {:reply, :ok, state}
  end

  @impl true
  def handle_message(message, state) do
    [_me | onward_route] = Message.onward_route(message)

    state
    |> Map.get(:routes, MapSet.new())
    |> Enum.each(fn {_name, route} ->
      ## TODO: forward_through
      Ockam.Router.route(Message.forward(message, route ++ onward_route))
    end)

    {:ok, state}
  end
end
