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
    :os.env()
    |> Enum.map(fn {name, val} -> {to_string(name), to_string(val)} end)
    |> Enum.filter(fn {name, _val} -> String.starts_with?(name, "SERVICE_PROXY_") end)
    |> Enum.map(fn {name, val} ->
      proxy_name = String.trim(name, "SERVICE_PROXY_")
      init_args = make_init_args(proxy_name, val, args)

      %{
        id: String.to_atom("proxy_#{proxy_name}"),
        start: {Proxy, :start_link, [init_args]}
      }
    end)
  end

  def make_init_args(proxy_name, val, args) do
    Keyword.merge(args,
      address: proxy_name,
      forward_route: val
    )
  end
end
