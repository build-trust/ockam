defmodule RemoteForwarder do
  alias Ockam.Router

  def create(route_to_forwarding_service, address_to_forward_to \\ nil, timeout \\ 60_000) do
    # Send 'register' message to forwarding service with the address to forward to in the return_route
    Router.route(%{
      onward_route: route_to_forwarding_service,
      return_route: [address_to_forward_to],
      payload: "register"
    })

    # Route to remote forwarder is the return_route of the reply
    receive do
      %{onward_route: [^address_to_forward_to], return_route: forwarder_route, payload: "register"} ->
        {:ok, List.last(forwarder_route)}
    after
      timeout ->
        {:error, :timeout}
    end
  end
end
