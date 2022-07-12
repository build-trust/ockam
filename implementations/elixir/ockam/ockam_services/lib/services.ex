defmodule Ockam.Services do
  # credo:disable-for-this-file Credo.Check.Refactor.ModuleDependencies

  @moduledoc """
  Main application to run Ockam Services

  Supervisor runs ockam services and transports
  """

  use Application

  alias Ockam.Services.Provider

  require Logger

  @doc false
  def start(_type, _args) do
    tcp_transport_options = Application.get_env(:ockam_services, :tcp_transport)
    udp_transport_options = Application.get_env(:ockam_services, :udp_transport)

    tcp_transport =
      case tcp_transport_options do
        nil -> []
        _options -> [{Ockam.Transport.TCP, tcp_transport_options}]
      end

    udp_transport =
      case udp_transport_options do
        nil -> []
        ## TODO: use same module format as TCP
        _options -> [{Ockam.Transport.UDP.Listener, udp_transport_options}]
      end

    children =
      tcp_transport ++
        udp_transport ++
        [
          Ockam.Services.Provider
        ]

    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end

  def start_service(name, options \\ []) do
    Provider.start_service({name, options}, Provider)
  end

  def stop_service(address) do
    Provider.stop_service(address, Provider)
  end
end
