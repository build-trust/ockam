defmodule Ockam.Message do
  @moduledoc """
  Message data structure for routing
  """
  defstruct [:payload, onward_route: [], return_route: [], version: 1]

  @type t() :: %__MODULE__{}

  def reply(message, my_address, payload) do
    %Ockam.Message{
      onward_route: return_route(message),
      return_route: [my_address],
      payload: payload
    }
  end

  def forward(message, forward_route) do
    %{message | onward_route: forward_route}
  end

  def forward(message, my_address, forward_route) do
    %{message | onward_route: forward_route, return_route: [my_address | return_route(message)]}
  end

  def onward_route(%Ockam.Message{onward_route: onward_route}), do: onward_route
  def return_route(%Ockam.Message{return_route: return_route}), do: return_route
  def payload(%Ockam.Message{payload: payload}), do: payload
end
