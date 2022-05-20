defmodule Ockam.Transport.TCP.RecoverableClient do
  @moduledoc """
  TCP client wrapper to recover connections.

  Creates and monitors Ockam.Transport.TCP.CLient
  If client stops, it's restarted after `refresh_timeout`

  Options:

  `desination` - Ockam.Transport.TCPAddress or {host, port} tuple to connect to
  `refresh_timeout` - time to wait between client restarts
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Transport.TCP.Client
  alias Ockam.Transport.TCPAddress

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def address_prefix(_options), do: "TCP_C_R_"

  @impl true
  def inner_setup(options, state) do
    destination_opt = Keyword.fetch!(options, :destination)
    refresh_timeout = Keyword.get(options, :refresh_timeout, 5_000)

    with {:ok, destination} <- make_destination(destination_opt) do
      state = Map.merge(state, %{destination: destination, refresh_timeout: refresh_timeout})

      {:ok, refresh_client(state)}
    end
  end

  @impl true
  def handle_inner_message(message, state) do
    [_me | onward_route] = Message.onward_route(message)
    [_client | return_route] = Message.return_route(message)
    payload = Message.payload(message)

    Router.route(%{
      onward_route: onward_route,
      return_route: [state.address | return_route],
      payload: payload
    })

    {:ok, state}
  end

  @impl true
  def handle_outer_message(message, %{client: client} = state) when client != nil do
    [_me | onward_route] = Message.onward_route(message)

    ## TODO: forward_through and trace
    Router.route(Message.forward_trace(message, [client | onward_route], state.inner_address))

    {:ok, state}
  end

  ## Ignore messages when client doesn't exist
  def handle_outer_message(_message, state) do
    {:ok, state}
  end

  @impl true
  def handle_info(:refresh_client, state) do
    {:noreply, refresh_client(state)}
  end

  def handle_info({:DOWN, ref, :process, _pid, _reason} = down, %{monitor_ref: ref} = state) do
    Logger.debug("DOWN for current client: #{inspect(down)} state: #{inspect(state)}")
    {:noreply, schedule_refresh_client(state)}
  end

  def handle_info({:DOWN, _ref, _type, _pid, _reason} = down, state) do
    Logger.debug("DOWN for old client: #{inspect(down)} state: #{inspect(state)}")
    {:noreply, state}
  end

  def refresh_client(state) do
    case Map.get(state, :client) do
      nil ->
        :ok

      client ->
        Ockam.Node.stop(client)
    end

    destination = Map.get(state, :destination)

    ## TODO: change monitors to links here
    case Client.create(destination: destination, restart_type: :temporary) do
      {:ok, client} ->
        monitor_client(client, state)

      {:error, _reason} ->
        schedule_refresh_client(state)
    end
  end

  def monitor_client(client, state) do
    case Ockam.Node.whereis(client) do
      nil ->
        schedule_refresh_client(state)

      pid ->
        monitor_ref = Process.monitor(pid)
        Map.merge(state, %{client: client, monitor_ref: monitor_ref})
    end
  end

  def schedule_refresh_client(state) do
    refresh_timeout = Map.fetch!(state, :refresh_timeout)
    timer_ref = Process.send_after(self(), :refresh_client, refresh_timeout)

    Map.put(state, :refresh_timer, timer_ref)
  end

  defp make_destination({_host, _port} = destination) do
    {:ok, destination}
  end

  defp make_destination(address) do
    TCPAddress.to_host_port(address)
  end
end
