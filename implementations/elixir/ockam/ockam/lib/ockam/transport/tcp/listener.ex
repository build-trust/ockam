if Code.ensure_loaded?(:ranch) do
  defmodule Ockam.Transport.TCP.Listener do
    @moduledoc """
    TCP listener GenServer for TCP transport
    Wrapper for ranch listener
    """

    ## TODO: is it possible to use ranch listener as a supervised process?
    use GenServer

    require Logger

    @typedoc """
    TCP listener options
    - ip: t::inet.ip_address() - IP address to listen on
    - port: t:integer() - port to listen on
    """
    @type options :: Keyword.t()

    def start_link(options) do
      GenServer.start_link(__MODULE__, options)
    end

    @doc false
    @impl true
    def init(options) do
      ip = Keyword.get_lazy(options, :ip, &default_ip/0)
      port = Keyword.get_lazy(options, :port, &default_port/0)

      handler_options = Keyword.get(options, :handler_options, [])

      ref = make_ref()
      transport = :ranch_tcp
      transport_options = [port: port, ip: ip]
      protocol = Ockam.Transport.TCP.Handler
      protocol_options = [packet: 2, nodelay: true, handler_options: handler_options]

      with {:ok, _apps} <- Application.ensure_all_started(:ranch),
           {:ok, ranch_listener} <-
             start_listener(ref, transport, transport_options, protocol, protocol_options) do
        {:ok, %{ranch_listener: ranch_listener}}
      end
    end

    defp start_listener(ref, transport, transport_options, protocol, protocol_options) do
      r = :ranch.start_listener(ref, transport, transport_options, protocol, protocol_options)

      case r do
        {:ok, child} -> {:ok, child}
        {:ok, child, _info} -> {:ok, child}
        {:error, reason} -> {:error, {:could_not_start_ranch_listener, reason}}
      end
    end

    def default_ip, do: {0, 0, 0, 0}
    def default_port, do: 4000
  end
end
