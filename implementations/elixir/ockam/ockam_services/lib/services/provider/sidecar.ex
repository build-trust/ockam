defmodule Ockam.Services.Provider.Sidecar do
  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing services on Rust sidecar

  Services:

  :identity_sidecar - sidecar service providing identity API for Ockam.Identity.Sidecar

  Options:

  - sidecar_host - hostname for sidecar node, default is "localhost"
  - sidecar_port - port for sidecar node, default is 4100
  - sidecar_address - worker address on the sidecar node, default is "identity_service"

  **NOTE** service address is used by Ockam.Identity.Sidecar
  and will always be as defined in Ockam.Identity.Sidecar.api_route()
  """
  @behaviour Ockam.Services.Provider

  @impl true
  def services() do
    [:identity_sidecar]
  end

  @impl true
  def child_spec(:identity_sidecar, args) do
    sidecar_host = Keyword.get(args, :sidecar_host, "localhost")
    sidecar_port = Keyword.get(args, :sidecar_port, 4100)
    sidecar_address = Keyword.get(args, :sidecar_address, "identity_service")
    forward_route = [Ockam.Transport.TCPAddress.new(sidecar_host, sidecar_port), sidecar_address]

    [api_address] = Ockam.Identity.Sidecar.api_route()

    extra_options =
      Keyword.drop(
        args,
        [:sidecar_port, :sidecar_host, :sidecar_address]
      )

    options = Keyword.merge(extra_options, forward_route: forward_route, address: api_address)

    %{
      id: __MODULE__,
      start: {Ockam.Services.Proxy, :start_link, [options]}
    }
  end
end
