defprotocol Ockam.Message do
  @moduledoc """
  Defines an elixir protocol for a message.
  """

  alias Ockam.Address
  alias Ockam.Serializable

  @fallback_to_any true

  @doc "Returns the onward_route of a message."
  @spec onward_route(t()) :: [Address.t()]
  def onward_route(message)

  @doc "Returns the return_route of a message."
  @spec return_route(t()) :: [Address.t()]
  def return_route(message)

  @doc "Returns the payload of a message."
  @spec payload(t()) :: Serializable.t()
  def payload(message)
end

# implement Ockam.Message for any message that does not already have an implementation
defimpl Ockam.Message, for: Any do
  @moduledoc false

  # if the message is a map that has an onward_route field with a list value, use it.
  def onward_route(%{onward_route: onward_route}) when is_list(onward_route), do: onward_route

  # for any other message, that does not implement Ockam.Message, assume onward_route is empty.
  def onward_route(_message), do: []

  # if the message is a map that has an return_route field with a list value, use it.
  def return_route(%{return_route: return_route}) when is_list(return_route), do: return_route

  # for any other message, that does not implement Ockam.Message, assume return_route is empty.
  def return_route(_message), do: []

  # if the message is a map that has an payload field, use it.
  def payload(%{payload: payload}), do: payload

  # for any other message, that does not implement Ockam.Message, assume the message is the payload.
  def payload(message), do: message
end
