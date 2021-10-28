defmodule Ockam.Transport.TCP.RecoverableClient do
  @moduledoc """
  TCP client wrapper to recover connections.

  Creates and monitors Ockam.Transport.TCP.CLient
  If client stops, it's restarted after `refresh_timeout`

  Options:

  `desination` - {host, port} tuple to connect to
  `refresh_timeout` - time to wait between client restarts
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Transport.TCP.Client

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def address_prefix(_options), do: "TCP_C_R_"

  @impl true
  def inner_setup(options, state) do
    destination = Keyword.fetch!(options, :destination)
    refresh_timeout = Keyword.get(options, :refresh_timeout, 5_000)

    state = Map.merge(state, %{destination: destination, refresh_timeout: refresh_timeout})

    {:ok, refresh_client(state)}
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

    Router.route(%{
      onward_route: [client | onward_route],
      return_route: [state.inner_address | Message.return_route(message)],
      payload: Message.payload(message)
    })

    {:ok, state}
  end

  ## Ignore messages when client doesn't exist
  def handle_outer_message(_message, state) do
    {:ok, state}
  end

  @impl true
  def handle_non_message(:refresh_client, state) do
    {:ok, refresh_client(state)}
  end

  def handle_non_message({:DOWN, ref, :process, _pid, _} = down, %{monitor_ref: ref} = state) do
    Logger.debug("DOWN for current client: #{inspect(down)} state: #{inspect(state)}")
    {:ok, schedule_refresh_client(state)}
  end

  def handle_non_message({:DOWN, _, _, _, _} = down, state) do
    Logger.debug("DOWN for old client: #{inspect(down)} state: #{inspect(state)}")
    {:ok, state}
  end

  def refresh_client(state) do
    case Map.get(state, :client) do
      nil ->
        :ok

      client ->
        Ockam.Node.stop(client)
    end

    destination = Map.get(state, :destination)

    case Client.create(destination: destination) do
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
end
