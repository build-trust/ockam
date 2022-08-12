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

  :ca_verifier_sidecar - sidecar service providing credential verifier for Ockam.Credential.Verifier.Sidecar

  Options:

  - sidecar_host - hostname for sidecar node, default is "localhost"
  - sidecar_port - port for sidecar node, default is 4100
  - sidecar_address - worker address on the sidecar node, default is "ca_verifier_service"

  :sidecar_node - service forwarding to a sidecar node (essentialy a persistent TCP client)

  Options:
  - service_id - atom id of the service for the supervisor
  - address - optional, address of the proxy worker, defaults to string service_id
  - sidecar_host - hostname for sidecar node, default is "localhost"
  - sidecar_port - port for sidecar node, default is 4100
  - refresh_timeout - heartbeat timeout to check the connection

  :sidecar_proxy - generic sidecar proxy service

  Options:
  - service_id - atom id of the service for the supervisor
  - address - optional, address of the proxy worker, defaults to string service_id
  - sidecar_host - hostname for sidecar node, default is "localhost"
  - sidecar_port - port for sidecar node, default is 4100
  - sidecar_address - worker address on the sidecar node
  """
  @behaviour Ockam.Services.Provider

  alias Ockam.Transport.TCP.RecoverableClient
  alias Ockam.Transport.TCPAddress

  @impl true
  def services() do
    [:identity_sidecar, :sidecar_proxy, :sidecar_node]
  end

  @impl true
  def child_spec(:identity_sidecar = service_id, args) do
    sidecar_address = Keyword.get(args, :sidecar_address, "identity_service")
    [address] = Ockam.Identity.Sidecar.api_route()

    child_spec(
      :sidecar_proxy,
      args ++ [service_id: service_id, address: address, sidecar_address: sidecar_address]
    )
  end

  def child_spec(:sidecar_proxy, args) do
    service_id = Keyword.fetch!(args, :service_id)
    address = Keyword.get(args, :address, to_string(service_id))

    sidecar_address = Keyword.fetch!(args, :sidecar_address)
    sidecar_host = Keyword.get(args, :sidecar_host, "localhost")
    sidecar_port = Keyword.get(args, :sidecar_port, 4100)
    forward_route = [TCPAddress.new(sidecar_host, sidecar_port), sidecar_address]

    extra_options =
      Keyword.drop(
        args,
        [:sidecar_port, :sidecar_host, :sidecar_address, :service_id]
      )

    options = Keyword.merge(extra_options, forward_route: forward_route, address: address)

    %{
      id: service_id,
      start: {Ockam.Services.Proxy, :start_link, [options]}
    }
  end

  def child_spec(:sidecar_node, args) do
    service_id = Keyword.fetch!(args, :service_id)
    address = Keyword.get(args, :address, to_string(service_id))

    sidecar_host = Keyword.get(args, :sidecar_host, "localhost")
    sidecar_port = Keyword.get(args, :sidecar_port, 4100)

    destination = TCPAddress.new(sidecar_host, sidecar_port)

    extra_options =
      Keyword.drop(
        args,
        [:sidecar_port, :sidecar_host, :service_id]
      )

    options = Keyword.merge(extra_options, destination: destination, address: address)

    %{
      id: service_id,
      start: {RecoverableClient, :start_link, [options]}
    }
  end
end
