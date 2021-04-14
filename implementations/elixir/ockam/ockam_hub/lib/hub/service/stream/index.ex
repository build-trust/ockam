defmodule Ockam.Hub.Service.Stream.Index do
  @moduledoc false

  use Ockam.MessageProtocol
  use Ockam.Worker

  require Logger

  @impl true
  def protocol_mapping() do
    Ockam.Protocol.server(Ockam.Protocol.Stream.Index)
  end

  @impl true
  def handle_message(%{payload: payload} = message, state) do
    case decode_payload(payload) do
      {:ok, _proto, {:save, data}} ->
        %{client_id: client_id, stream_name: stream_name, index: index} = data
        Logger.info("Save index #{inspect({client_id, stream_name, index})}")
        save_index({client_id, stream_name}, index, state)

      {:ok, _proto, {:get, data}} ->
        %{client_id: client_id, stream_name: stream_name} = data
        Logger.info("get index #{inspect({client_id, stream_name})}")
        index = get_index({client_id, stream_name}, state)
        reply_index(client_id, stream_name, index, Ockam.Message.return_route(message), state)
        {:ok, state}

      {:error, other} ->
        Logger.error("Unexpected message #{inspect(other)}")
        {:ok, state}
    end
  end

  def save_index(id, index, state) do
    indices = Map.get(state, :indices, %{})

    new_indices = Map.update(indices, id, index, fn previous -> max(previous, index) end)

    {:ok, Map.put(state, :indices, new_indices)}
  end

  def get_index(id, state) do
    state
    |> Map.get(:indices, %{})
    |> Map.get(id, 0)
  end

  def reply_index(client_id, stream_name, index, return_route, state) do
    Ockam.Router.route(%{
      onward_route: return_route,
      return_route: [state.address],
      payload:
        encode_payload("stream_index", %{
          client_id: client_id,
          stream_name: stream_name,
          index: index
        })
    })
  end
end
