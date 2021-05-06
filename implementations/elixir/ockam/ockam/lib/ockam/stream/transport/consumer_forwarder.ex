defmodule Ockam.Stream.Transport.ConsumerForwarder do
  @moduledoc """
  Forwarding module to subscribe for HUB streams
  TODO: Stream protocol should be moved to the edges ideally and this module removed
  """

  ## TODO: maybe send publishers through this as well to have a first message routes updated
  use Ockam.Worker

  alias Ockam.Stream.Client.Consumer
  alias Ockam.Stream.Transport.Address

  alias Ockam.Message
  alias Ockam.Node
  alias Ockam.Router

  require Logger

  def subscribe(route, options) do
    with {:ok, address} <- __MODULE__.create([forward_route: route] ++ options) do
      Consumer.create(
        [
          message_handler: fn data ->
            forward_consumer_message(data, address, route)
          end
        ] ++ options
      )
    end
  end

  def forward_consumer_message(data, address, forward_route) do
    {:ok, transport_message} = Ockam.Stream.Transport.decode_transport_message(data)

    Logger.info("Forwarded message #{inspect(transport_message)} to #{inspect(address)}")

    case forward_route do
      [] ->
        ## Optimizing for when there is no forward route
        Router.route(transport_message)

      _other ->
        ## TODO: maybe cut the corner here by routing consumed messages directly to forward
        Node.send(address, transport_message)
    end
  end

  @impl true
  def setup(options, state) do
    forward_route = Keyword.get(options, :forward_route, [])
    stream_name = Keyword.fetch!(options, :stream_name)

    {:ok, Map.merge(state, %{forward_route: forward_route, stream_name: stream_name})}
  end

  @impl true
  def handle_message(message, state) do
    return_route = Message.return_route(message)
    stream_name = Map.fetch!(state, :stream_name)

    # This is a bit hacky
    case return_route do
      [%Address{return_stream: ^stream_name} | _] ->
        ## Consumed message
        handle_consumed_message(message, state)

      _other ->
        handle_published_message(message, state)
    end
  end

  def handle_published_message(message, state) do
    Logger.info("Published message #{inspect(message)}")
    return_route = Message.return_route(message)
    forward_route = Map.get(state, :forward_route, [])
    self_address = Map.fetch!(state, :address)
    [^self_address | onward_route] = Message.onward_route(message)

    updated_return_route =
      case List.starts_with?(return_route, forward_route) do
        true ->
          Enum.drop(return_route, Enum.count(forward_route))

        false ->
          Logger.error(
            "Forward route #{inspect(forward_route)} should be a prefix of return route: #{
              inspect(return_route)
            }"
          )

          return_route
      end

    routed_message = %{message | return_route: updated_return_route}

    Logger.info("Published updated message: #{inspect(routed_message)}")

    Router.route(%{message | return_route: updated_return_route, onward_route: onward_route})
  end

  def handle_consumed_message(message, state) do
    Logger.info("Consumer message #{inspect(message)}")
    forward_route = Map.get(state, :forward_route, [])
    onward_route = Message.onward_route(message)
    return_route = Message.return_route(message)

    ## Here is an ugly part.
    onward_route =
      case List.starts_with?(onward_route, forward_route) do
        true -> Enum.drop(onward_route, Enum.count(forward_route))
        false -> onward_route
      end

    self_address = Map.fetch!(state, :address)

    routed_message = %{
      message
      | onward_route: forward_route ++ onward_route,
        return_route: [self_address | return_route]
    }

    Logger.info("Routed message #{inspect(routed_message)}")

    Router.route(routed_message)
  end
end
