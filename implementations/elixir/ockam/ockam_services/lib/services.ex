defmodule Ockam.Services do
  # credo:disable-for-this-file Credo.Check.Refactor.ModuleDependencies

  @moduledoc """
  Main application to run Ockam Services

  Supervisor runs ockam services and transports
  """

  use Application

  require Logger

  @doc false
  def start(_type, _args) do
    tcp_transport_port = Application.get_env(:ockam_services, :tcp_transport_port, 4000)
    udp_transport_port = Application.get_env(:ockam_services, :udp_transport_port, 7000)

    children = [
      # Add a TCP listener
      {Ockam.Transport.TCP, [listen: [port: tcp_transport_port]]},
      # Add a UDP listener
      ## TODO: use same module format as TCP
      {Ockam.Transport.UDP.Listener,
       [
         port: udp_transport_port
       ]},
      Ockam.Services.Provider
    ]

    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end

  def start_service(name, options \\ []) do
    Ockam.Services.Provider.start_service({name, options}, __MODULE__)
  end
end
