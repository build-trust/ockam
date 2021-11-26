defmodule Ockam.Message do
  @moduledoc """
  Message data structure for routing
  """
  defstruct [:payload, onward_route: [], return_route: [], version: 1]

  @type t() :: %__MODULE__{}

  @doc """
  Creates a message to reply for `message`

  onward_route is a return route from `message`
  return_route is `[my_address]`
  """
  def reply(message, my_address, payload) do
    %Ockam.Message{
      onward_route: return_route(message),
      return_route: [my_address],
      payload: payload
    }
  end

  @doc """
  Forward to the next address in the onward route
  """
  def forward(%Ockam.Message{} = message) do
    [_me | onward_route] = onward_route(message)
    %{message | onward_route: onward_route}
  end

  @doc """
  Forward to a specified route
  """
  def forward(%Ockam.Message{} = message, route) when is_list(route) do
    %{message | onward_route: route}
  end

  @doc """
  Trace `address` in the return route
  """
  def trace_address(%Ockam.Message{} = message, address) do
    %{message | return_route: [address | return_route(message)]}
  end

  @doc """
  Forward to the next address in the onward route and trace
  the current address in the return route
  """
  def forward_trace(%Ockam.Message{} = message) do
    [me | onward_route] = onward_route(message)
    message |> forward(onward_route) |> trace_address(me)
  end

  @doc """
  Forward to the next address in the onward route and trace
  the `address` in the return route
  """
  def forward_trace(%Ockam.Message{} = message, address) do
    message |> forward() |> trace_address(address)
  end

  @doc """
  Forward to the specified `route` and trace
  the `address` in the return route
  """
  def forward_trace(%Ockam.Message{} = message, route, address) do
    message |> forward(route) |> trace_address(address)
  end

  @doc "Get onward_route from the message"
  def onward_route(%Ockam.Message{onward_route: onward_route}), do: onward_route

  @doc "Get return_route from the message"
  def return_route(%Ockam.Message{return_route: return_route}), do: return_route

  @doc "Get payload from the message"
  def payload(%Ockam.Message{payload: payload}), do: payload
end
