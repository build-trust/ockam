defprotocol Ockam.Router.Message do
  @moduledoc """
  Defines an elixir protocol for a message that Ockam.Router can route.
  """
  @fallback_to_any true

  @doc "Returns the onward_route of a message."
  @spec onward_route(t()) :: Ockam.Router.Route.t()
  def onward_route(message)
end

# implement Ockam.Router.Message for any message that does not already have an implementation
defimpl Ockam.Router.Message, for: Any do
  # if the message is a map that has an onward_route field with a list value, use it.
  def onward_route(%{onward_route: onward_route}) when is_list(onward_route), do: onward_route

  # for any other message, that does not implement Ockam.Router.Message, assume onward_route is empty.
  def onward_route(_message), do: []
end
