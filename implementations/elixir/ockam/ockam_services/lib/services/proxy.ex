defmodule Ockam.Services.Proxy do
  @moduledoc """
  Proxy worker to set up static aliases to workers
  (potentially on other nodes)

  Forwards messages to the `forward_route`, tracing own inner_address in the return route
  Forwards messages from the inner address, replacing return route with own outer address

  -OR:[outer_address],RR:return_route-> proxy -OR:forward_route,RR:[inner_address] ; return_route->
  -OR:[inner_address] ; onward_route,RR:return_route-> proxy -OR:onward_route;RR:[outer_address]->

  Limitations:
  - proxy worker address should be terminal in the forwarding messages, onward_route will be ignored
  - return route of inner messages is replaced with the proxy outer address

  Due to these limitations, it may be not possible to establish sessions over the proxy workers.
  Please use `Ockam.Services.Forwarding` or `Ockam.Services.StaticForwarding` to set up sessions.

  Forwarding to TCP addresses:

  If the first address of the route is TCP, forwarding might cause TCP clients leak, because each
  new message will create a new TCP client.

  To prevent that, if the first address is TCP, a `Ockam.Transport.TCP.RecoverableClient`
  instance is created, and the TCP address is replaced with this client address

  Options:

  - forward_route: route (list or string formatted) to forward messages to
  """

  use Ockam.AsymmetricWorker

  alias Ockam.Address
  alias Ockam.Message
  alias Ockam.Worker

  alias Ockam.Transport.TCP.RecoverableClient
  alias Ockam.Transport.TCPAddress

  @impl true
  def inner_setup(options, state) do
    forward_route_option = Keyword.fetch!(options, :forward_route)

    with {:ok, [first_address | route_tail]} <- forward_route_config(forward_route_option) do
      case TCPAddress.is_tcp_address(first_address) do
        true ->
          client_address = "PROXY_CLIENT_" <> state.address

          inner_address = state.inner_address
          client_auth = %{client_address => [from_addresses: [:message, [inner_address]]]}

          with {:ok, _pid, client_address} <-
                 RecoverableClient.start_link(
                   destination: first_address,
                   address: client_address,
                   authorization: client_auth
                 ) do
            forward_route = [client_address | route_tail]

            ## Only authorize inner address to accept messages from proxy client
            ## TODO: we need better authorization mechanism
            state =
              Worker.update_authorization_state(state, inner_address,
                from_addresses: [:message, [client_address]]
              )

            {:ok,
             Map.merge(state, %{
               forward_route: forward_route,
               client_address: client_address
             })}
          end

        false ->
          forward_route = [first_address | route_tail]
          {:ok, Map.merge(state, %{forward_route: forward_route})}
      end
    end
  end

  def forward_route_config(string) when is_binary(string) do
    forward_route_config(Address.parse_route!(string))
  catch
    _type, _error ->
      {:error, :cannot_parse_forward_route}
  end

  def forward_route_config([]) do
    {:error, :forward_route_cannot_be_empty}
  end

  def forward_route_config(route) when is_list(route) do
    {:ok, route}
  end

  @impl true
  def handle_outer_message(message, state) do
    forward_route = Map.get(state, :forward_route)
    inner_address = Map.get(state, :inner_address)

    forwarded_message =
      Message.set_onward_route(message, forward_route) |> Message.trace(inner_address)

    Worker.route(forwarded_message, state)
    {:ok, state}
  end

  @impl true
  def handle_inner_message(message, state) do
    outer_address = Map.get(state, :address)
    return_route = [outer_address]

    forwarded_message = Message.forward(message) |> Map.put(:return_route, return_route)

    Worker.route(forwarded_message, state)
    {:ok, state}
  end
end
