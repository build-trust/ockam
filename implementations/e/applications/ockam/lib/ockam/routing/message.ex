defprotocol Ockam.Routing.Message do

  @fallback_to_any true

  @spec onward_route(t) :: Ockam.Routing.Route.t()
  def onward_route(message)

  @spec return_route(t) :: Ockam.Routing.Route.t()
  def return_route(message)

  @spec payload(t) :: any()
  def payload(message)

end

defimpl Ockam.Routing.Message, for: Any do

  def onward_route(%{onward_route: onward_route}) when is_list(onward_route), do: onward_route
  def onward_route(_message), do: []

  def return_route(%{return_route: return_route}) when is_list(return_route), do: return_route
  def return_route(_message), do: []

  def payload(%{payload: payload}), do: payload
  def payload(message), do: message

end
