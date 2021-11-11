defmodule Ockam.Session.Handshake.Default do
  @moduledoc """
  Simple handshake logic using an empty message
  and return route tracing
  """

  @behaviour Ockam.Session.Handshake

  alias Ockam.Message

  def init(_options, state) do
    init_route = Map.fetch!(state, :init_route)

    {:next,
     %{
       onward_route: init_route,
       return_route: [state.handshake_address],
       payload: :bare.encode(state.worker_address, :string)
     }, state}
  end

  def handle_initiator(_options, message, state) do
    return_route = Message.return_route(message)
    handshake_data = Message.payload(message)
    ## TODO: use special data types?
    case :bare.decode(handshake_data, :string) do
      {:ok, worker_address, ""} ->
        worker_route = make_remote_worker_route(worker_address, return_route)
        {:ready, [route: worker_route], state}

      err ->
        {:error, {:invalid_handshake_message, message, err}}
    end
  end

  def handle_responder(_options, message, state) do
    return_route = Message.return_route(message)
    handshake_data = Message.payload(message)

    case :bare.decode(handshake_data, :string) do
      {:ok, worker_address, ""} ->
        worker_route = make_remote_worker_route(worker_address, return_route)

        response = %{
          onward_route: return_route,
          return_route: [state.handshake_address],
          payload: :bare.encode(state.worker_address, :string)
        }

        {:ready, response, [route: worker_route], state}

      err ->
        {:error, {:invalid_handshake_message, message, err}}
    end
  end

  def make_remote_worker_route(address, route) do
    Enum.take(route, Enum.count(route) - 1) ++ [address]
  end
end
