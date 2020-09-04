defprotocol Ockam.Router.Message do
  @moduledoc """
  Defines an elixir protocol for a address that Ockam.Router can route.
  """

  @fallback_to_any true

  @typedoc "A route is an ordered list of addresses."
  @type route :: [Ockam.Router.Address.t()]

  @doc """
  Returns the onward_route of a message
  """
  @spec onward_route(t) :: route
  def onward_route(message)
end

defimpl Ockam.Router.Message, for: Any do
  def onward_route(%{onward_route: onward_route}) when is_list(onward_route), do: onward_route
  def onward_route(_message), do: []
end

defmodule Ockam.Router.MessageHandler do
  @moduledoc false
  @type result :: :ok | {:error, reason :: any()}
  @type t :: (Ockam.Router.Message.t() -> result())
end
