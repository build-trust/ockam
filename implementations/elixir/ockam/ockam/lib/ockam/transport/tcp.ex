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
        ip = Keyword.get(listen, :ip, Listener.default_ip())
        port = Keyword.get(listen, :port, Listener.default_port())

        "TCP_LISTENER_#{TCPAddress.format_host_port(ip, port)}"

      _other ->
        "TCP_TRANSPORT"
    end
  end

  @doc """
  Start a TCP transport

  ## Parameters
  - options:
      listen: t:Listener.options()
  """
  @spec start(Keyword.t()) :: :ignore | {:error, any} | {:ok, any}
  def start(options \\ []) do
    client_options = Keyword.get(options, :client_options, [])
    ## TODO: do we want to stop transports?
    Router.set_message_handler(
      TCPAddress.type(),
      {__MODULE__, :handle_transport_message, [client_options]}
    )

    case Keyword.fetch(options, :listen) do
      {:ok, listen} -> Listener.start_link(listen)
      _other -> :ignore
    end
  end

  @spec handle_transport_message(Ockam.Message.t(), Keyword.t()) :: :ok | {:error, any()}
  def handle_transport_message(message, client_options) do
    case get_destination(message) do
      {:ok, destination} ->
        ## TODO: reuse clients when using tcp address
        with {:ok, client_address} <-
               Client.create([
                 {:destination, destination},
                 {:restart_type, :temporary} | client_options
               ]) do
          Ockam.Node.send(client_address, message)
        end

      e ->
        Logger.error(
          "Cannot forward message to tcp client: #{inspect(message)} reason: #{inspect(e)}"
        )
    end
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
