defmodule Ockam.Workers.PubSubSubscriber do
  @moduledoc """
  Worker to maintain Ockam.Services pub_sub service subscription

  Refreshes subscription to pub_sup service every interval milliseconds

  Can be used to maintain resilient subscription to a topic
  even if the connection or pub_sub service restarts

  Options:

  `pub_sub_route` - route to pub_sub service
  `name` - subscription name
  `topic` - subscription topic
  `interval` - interval to refresh subscription, defaults to 10 seconds
  """

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Worker

  require Logger

  @impl true
  def address_prefix(_options), do: "PS_"

  @impl true
  def setup(options, state) do
    pub_sub_route = Keyword.fetch!(options, :pub_sub_route)
    topic = Keyword.fetch!(options, :topic)
    name = Keyword.fetch!(options, :name)
    interval = Keyword.get(options, :interval, 10_000)

    state =
      Map.merge(state, %{
        pub_sub_route: pub_sub_route,
        interval: interval,
        topic: topic,
        name: name
      })

    {:ok, refresh_subscription(state)}
  end

  @impl true
  def handle_message(%{payload: _} = message, %{name: name, topic: topic} = state) do
    [_me | onward_route] = Message.onward_route(message)

    case onward_route do
      [] ->
        expected_ok = :bare.encode("#{name}:#{topic}", :string)

        case Message.payload(message) do
          ^expected_ok ->
            ## TODO: don't forward messages before getting this
            Logger.debug("Subscribed to topic: #{topic} with name: #{name}")

          _other ->
            Logger.info("Unexpected message: #{inspect(message)} #{inspect(state)}")
        end

        :ok

      route ->
        Worker.route(Message.set_onward_route(message, route), state)
    end

    {:ok, state}
  end

  @impl true
  def handle_info(:refresh, state) do
    {:noreply, refresh_subscription(state)}
  end

  def refresh_subscription(state) do
    pub_sub_route = Map.fetch!(state, :pub_sub_route)
    topic = Map.fetch!(state, :topic)
    name = Map.fetch!(state, :name)
    interval = Map.fetch!(state, :interval)

    Worker.route(
      %{
        onward_route: pub_sub_route,
        return_route: [state.address],
        payload: :bare.encode(name <> ":" <> topic, :string)
      },
      state
    )

    Process.send_after(self(), :refresh, interval)
    state
  end
end
