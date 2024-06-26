if Code.ensure_loaded?(:ranch) do
  defmodule Ockam.Transport.TCP.Listener do
    @moduledoc """
    TCP listener GenServer for TCP transport
    Wrapper for ranch listener
    """

    ## TODO: is it possible to use ranch listener as a supervised process?
    require Logger

    @typedoc """
    TCP listener options
    - ip: t::inet.ip_address() - IP address to listen on
    - port: t:integer() - port to listen on
    """
    @type options :: Keyword.t()

    def start_link(options) do
      ip = Keyword.get_lazy(options, :ip, &default_ip/0)
      port = Keyword.get_lazy(options, :port, &default_port/0)

      ref = Keyword.get_lazy(options, :ref, &make_ref/0)

      handler_options = Keyword.get(options, :handler_options, [])

      transport = :ranch_tcp
      transport_options = :ranch.normalize_opts(port: port, ip: ip)
      protocol = Ockam.Transport.TCP.Handler
      protocol_options = [nodelay: true, handler_options: handler_options]

      with {:ok, _apps} <- Application.ensure_all_started(:ranch) do
        {:ok, listener_address} =
          start_listener(ref, transport, transport_options, protocol, protocol_options)

        :ranch.set_max_connections(ref, 1_000_000)
        {:ok, listener_address}
      end
    end

    defp start_listener(ref, transport, transport_options, protocol, protocol_options) do
      :ranch_listener_sup.start_link(
        ref,
        transport,
        transport_options,
        protocol,
        protocol_options
      )
    end

    def default_ip, do: {0, 0, 0, 0}
    def default_port, do: 4000

    # Get the port used by the listener 'ref'.  Used on tests cases to listen on random
    # unused ports.  Only really useful when the ref is passed explicitly at creation time.
    def get_port(ref) do
      {:ok, :ranch.get_port(ref)}
    end
  end
end
