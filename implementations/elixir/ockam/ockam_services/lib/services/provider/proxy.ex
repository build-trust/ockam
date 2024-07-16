defmodule Ockam.Services.Provider.Proxy do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing the proxy service which can be used
  to set up proxies to remote services

  Configuration:
  "SERVICE_PROXY_{name}"="route_to_remote_service"

  For example:
  "SERVICE_PROXY_remote_echo"="1#localhost:4000;0#echo"
  """
  @behaviour Ockam.Services.Provider

  alias Ockam.Services.Proxy

  @impl true
  def services() do
    [:proxy]
  end

  @impl true
  def child_spec(:proxy, args) do
    [
      %{
        id: String.to_atom("proxy_#{Keyword.fetch!(args, :address)}"),
        start: {Proxy, :start_link, [args]}
      }
    ]
  end
end
