defmodule Ockam.Message do
  @moduledoc """
  Message data structure for routing
  """
  alias Ockam.Message

  defstruct [:payload, onward_route: [], return_route: [], version: 1, local_metadata: %{}]

  @type t() :: %__MODULE__{}

  @doc """
  Creates a message to reply for `message`

  onward_route is a return route from `message`
  return_route is `[my_address]`
  """
  def reply(message, my_address, payload) do
    %Message{
      onward_route: return_route(message),
      return_route: [my_address],
      payload: payload
    }
  end

  @doc """
  Forward to the next address in the onward route
  """
  def forward(%Message{} = message) do
    [_me | onward_route] = onward_route(message)
    set_onward_route(message, onward_route)
  end

  @doc """
  Trace `address` in the return route
  """
  def trace(%Message{} = message, address) do
    set_return_route(message, [address | return_route(message)])
  end

  @doc """
  Forward to the next address in the onward route and trace
  the current address in the return route
  """
  def forward_trace(%Message{} = message) do
    [me | onward_route] = onward_route(message)
    message |> set_onward_route(onward_route) |> trace(me)
  end

  @doc "Get onward_route from the message"
  def onward_route(%Message{onward_route: onward_route}) when is_list(onward_route),
    do: onward_route

  @doc "Get return_route from the message"
  def return_route(%Message{return_route: return_route}) when is_list(return_route),
    do: return_route

  def return_route(%Message{return_route: nil}), do: []

  @doc "Get payload from the message"
  def payload(%Message{payload: payload}), do: payload

  @doc "Get local metadata from the message"
  def local_metadata(%Message{local_metadata: local_metadata}), do: local_metadata

  @doc "Get local metadata value for key from the message"
  def local_metadata_value(%Message{local_metadata: local_metadata}, key) do
    Map.get(local_metadata, key)
  end

  def set_onward_route(%Message{} = message, onward_route) when is_list(onward_route) do
    %{message | onward_route: onward_route}
  end

  def set_return_route(%Message{} = message, return_route) when is_list(return_route) do
    %{message | return_route: return_route}
  end

  def set_payload(%Message{} = message, payload) when is_list(payload) do
    %{message | payload: payload}
  end

  def set_local_metadata(%Message{} = message, metadata) when is_map(metadata) do
    %{message | local_metadata: metadata}
  end

  def put_local_metadata(%Message{} = message, key, value) when is_atom(key) do
    Map.update(message, :local_metadata, %{key => value}, fn metadata ->
      Map.put(metadata, key, value)
    end)
  end
end
