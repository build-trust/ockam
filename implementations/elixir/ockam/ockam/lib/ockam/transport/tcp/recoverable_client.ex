defmodule Ockam.Transport.TCP.RecoverableClient do
  @moduledoc """
  TCP client wrapper to recover connections.

  Creates and monitors Ockam.Transport.TCP.Client
  If client stops, it's restarted after `refresh_timeout`

  Options:

  `destination` - Ockam.Transport.TCPAddress or {host, port} tuple to connect to
  `refresh_timeout` - time to wait between client restarts
  """
  use Ockam.AsymmetricWorker

  alias Ockam.Transport.TCP.Client
  alias Ockam.Transport.TCPAddress

  alias Ockam.Message
  alias Ockam.Worker

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
  def handle_inner_message(message, %{client: client} = state) do
    [source_client | return_route] = Message.return_route(message)

    case source_client do
      ^client ->
        forwarded_message =
          message
          |> Message.forward()
          |> Message.set_return_route([state.address | return_route])

        Worker.route(forwarded_message, state)

        {:ok, state}

      _other ->
        ## We can only accept messages from the current client on the inner address
        {:error, {:invalid_inner_address_client, source_client}}
    end
  end

  @impl true
  def handle_outer_message(message, %{client: client} = state) when client != nil do
    [_me | onward_route] = Message.onward_route(message)

    ## TODO: forward_through and trace
    Worker.route(
      Message.set_onward_route(message, [client | onward_route])
      |> Message.trace(state.inner_address),
      state
    )

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

    inner_address = state.inner_address

    client_authorization = [from_addresses: [:message, [inner_address]]]

    ## TODO: change monitors to links here
    case Client.create(
           destination: destination,
           restart_type: :temporary,
           authorization: client_authorization
         ) do
      {:ok, client} ->
        inner_authorization = [from_addresses: [:message, [client]]]
        state = Ockam.Worker.update_authorization_state(state, inner_address, inner_authorization)
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
