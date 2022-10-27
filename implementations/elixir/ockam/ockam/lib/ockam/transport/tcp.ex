defmodule Ockam.Transport.TCP do
  @moduledoc """
  TCP transport
  """

  alias Ockam.Transport.TCP.Listener
  alias Ockam.Transport.TCPAddress

  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Transport.TCP.Client

  require Logger

  @packed_size_limit 65_000

  def packed_size_limit() do
    @packed_size_limit
  end

  def child_spec(options) do
    id = id(options)

    %{
      id: id,
      start: {__MODULE__, :start, [options]}
    }
  end

  defp id(options) do
    case Keyword.fetch(options, :listen) do
      {:ok, listen} ->
        if Code.ensure_loaded(:ranch) do
          ip = Keyword.get(listen, :ip, Listener.default_ip())
          port = Keyword.get(listen, :port, Listener.default_port())

          "TCP_LISTENER_#{TCPAddress.format_host_port(ip, port)}"
        else
          "TCP_TRANSPORT"
        end

      _other ->
        "TCP_TRANSPORT"
    end
  end

  ## TODO: rename to start_link
  @doc """
  Start a TCP transport

  ## Parameters
  - options:
      listen: t:Listener.options() - TCP listener options, default is empty (no listener is started)
      implicit_clients: boolean() - start client on receiving TCPAddress message, default is true
      client_options: list() - additional options to pass to implicit clients
  """
  @spec start(Keyword.t()) :: :ignore | {:error, any} | {:ok, any}
  def start(options \\ []) do
    client_options = Keyword.get(options, :client_options, [])
    implicit_clients = Keyword.get(options, :implicit_clients, true)

    case implicit_clients do
      true ->
        ## TODO: do we want to stop transports?
        Router.set_message_handler(
          TCPAddress.type(),
          {__MODULE__, :handle_transport_message, [client_options]}
        )

      false ->
        Router.set_message_handler(
          TCPAddress.type(),
          {__MODULE__, :implicit_connections_disabled, []}
        )
    end

    case Keyword.fetch(options, :listen) do
      {:ok, listen} ->
        if Code.ensure_loaded(:ranch) do
          Listener.start_link(listen)
        else
          {:error, :ranch_not_loaded}
        end

      _other ->
        :ignore
    end
  end

  @spec handle_transport_message(Ockam.Message.t(), Keyword.t()) :: :ok | {:error, any()}
  def handle_transport_message(message, client_options) do
    case get_destination(message) do
      {:ok, destination} ->
        case Client.create([
               {:destination, destination},
               {:restart_type, :temporary} | client_options
             ]) do
          {:ok, client_address} ->
            [_tcp_address | onward_route] = Message.onward_route(message)
            Router.route(Message.set_onward_route(message, [client_address | onward_route]))

          {:error, {:worker_init, _worker, reason}} ->
            {:error, reason}

          {:error, reason} ->
            {:error, reason}
        end

      e ->
        Logger.error(
          "Cannot forward message to tcp client: #{inspect(message)} reason: #{inspect(e)}"
        )
    end
  end

  def implicit_connections_disabled(_message) do
    {:error, {:tcp_transport, :implicit_connections_disabled}}
  end

  defp get_destination(message) do
    [dest_address | _onward_route] = Message.onward_route(message)

    with true <- TCPAddress.is_tcp_address(dest_address),
         {:ok, destination} <- TCPAddress.to_host_port(dest_address) do
      {:ok, destination}
    else
      false ->
        {:error, {:invalid_address_type, dest_address}}

      {:error, error} ->
        {:error, error}
    end
  end
end
