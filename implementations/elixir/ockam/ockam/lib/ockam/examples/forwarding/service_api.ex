defmodule Ockam.Examples.Forwarding.ServiceApi do
  @moduledoc """
  Simple forwarding service client.
  Registers current process with a remote forwarder in ockam services node.
  Current process needs to be registered in Ockam.Node.Registry
  """

  @type address() :: Ockam.Address.t()

  @doc """
  Register current process with `self_address` with a remote forwarding service
  accessed by `forwarding_service_route`

  `forwarding_service_route` - a route to forwarding service
  `self_address` - local address of this process, needs to resolve to `self()`, if nil - gets an address from the Ockam.Node.Registry pointing to `self()`
  `timeout` - registration timeout

  Returns:

  {:ok, forwarder_address} - an address on services node, forwarding to the current worker
  {:error, :timeout} - timeout before registration confirmation is received
  {:error, :not_registered_worker} - self_address is nil and current process is not registered in Ockam.Node.Registry
  """
  @spec register_self([address()], address() | nil, integer) ::
          {:ok, [address()]} | {:error, any()}

  def register_self(forwarding_service_route, self_address \\ nil, timeout \\ 60_000) do
    with {:ok, self_address} <- resolve_self_address(self_address) do
      msg = %{
        onward_route: forwarding_service_route,
        return_route: [self_address],
        payload: "register"
      }

      Ockam.Router.route(msg)

      receive do
        %{onward_route: [^self_address], return_route: forwarder_route, payload: "register"} ->
          {:ok, List.last(forwarder_route)}
      after
        timeout ->
          {:error, :timeout}
      end
    end
  end

  defp resolve_self_address(nil) do
    case Ockam.Node.list_addresses(self()) do
      [] -> {:error, :not_registered_worker}
      [address | _] -> {:ok, address}
    end
  end

  defp resolve_self_address(address) do
    {:ok, address}
  end
end
