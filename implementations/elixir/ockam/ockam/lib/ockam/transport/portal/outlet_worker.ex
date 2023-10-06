defmodule Ockam.Transport.Portal.OutletWorker do
  @moduledoc """
  Portal protocol, Outlet worker
  """

  use Ockam.Worker
  alias Ockam.Message
  alias Ockam.Transport.Portal.TunnelProtocol
  alias Ockam.Worker

  require Logger

  @impl true
  def setup(options, state) do
    msg = options[:init_message]
    target_host = options[:target_host] |> to_charlist()
    target_port = options[:target_port]

    ssl = Keyword.get(options, :ssl, false)
    ssl_options = Keyword.get(options, :ssl_options, [])
    tcp_wrapper = Keyword.get(options, :tcp_wrapper, Ockam.Transport.TCP.DefaultWrapper)

    Logger.info(
      "Starting outlet worker to #{target_host}:#{target_port}.  peer: #{inspect(msg.return_route)}"
    )

    ## TODO: support connect timeout
    timeout = :infinity

    with {:ok, :ping} <- TunnelProtocol.decode(msg.payload),
         # TODO: wrap TCP connect?
         {:ok, socket} <-
           :gen_tcp.connect(target_host, target_port, [{:active, :once}, :binary], timeout),
         {:ok, socket} <- maybe_upgrade_to_ssl(socket, ssl, ssl_options, timeout) do
      Process.flag(:trap_exit, true)
      :ok = Worker.route(Message.reply(msg, state.address, TunnelProtocol.encode(:pong)), state)

      protocol =
        case ssl do
          true ->
            %{
              inet_mod: :ssl,
              send_mod: :ssl,
              data_tag: :ssl,
              error_tag: :ssl_error,
              closed_tag: :ssl_closed
            }

          false ->
            %{
              inet_mod: :inet,
              send_mod: :gen_tcp,
              data_tag: :tcp,
              error_tag: :tcp_error,
              closed_tag: :tcp_closed
            }
        end

      {:ok,
       state
       |> Map.put(:socket, socket)
       |> Map.put(:protocol, protocol)
       |> Map.put(:peer, msg.return_route)
       |> Map.put(:connected, true)
       |> Map.put(:tcp_wrapper, tcp_wrapper)}
    else
      error ->
        Logger.error("Error starting outlet: #{inspect(options)} : #{inspect(error)}")

        :ok =
          Worker.route(
            Message.reply(msg, state.address, TunnelProtocol.encode(:disconnect)),
            state
          )

        {:error, :normal}
    end
  end

  @impl true
  def handle_message(%Message{payload: msg_data}, state) do
    with {:ok, protocol_msg} <- TunnelProtocol.decode(msg_data) do
      handle_protocol_msg(state, protocol_msg)
    end
  end

  @impl true
  def handle_info(
        {data_tag, socket, data},
        %{peer: peer, protocol: %{data_tag: data_tag, inet_mod: inet_mod}} = state
      ) do
    :ok =
      Worker.route(
        %Message{payload: TunnelProtocol.encode({:payload, data}), onward_route: peer},
        state
      )

    inet_mod.setopts(socket, active: :once)
    {:noreply, state}
  end

  def handle_info({closed_tag, _socket}, %{protocol: %{closed_tag: closed_tag}} = state) do
    Logger.info("Socket closed")
    {:stop, :normal, state}
  end

  def handle_info({error_tag, _socket, reason}, %{protocol: %{error_tag: error_tag}} = state) do
    Logger.info("Socket error: #{inspect(reason)}")
    {:stop, {:error, reason}, state}
  end

  ## We need to trap exits to cleanup tcp connection.
  ## If the connection port terminates with :normal - we still need to stop the outlet
  def handle_info({:EXIT, socket, :normal}, %{socket: socket} = state) do
    {:stop, :socket_terminated, state}
  end

  ## Linked processes terminating normally should not stop the outlet.
  ## Technically this should not happen
  def handle_info({:EXIT, from, :normal}, state) do
    Logger.warning("Received exit :normal signal from #{inspect(from)}")
    {:noreply, state}
  end

  def handle_info({:EXIT, _from, reason}, state) do
    {:stop, reason, state}
  end

  @impl true
  def terminate(reason, %{peer: peer, connected: true} = state) do
    Logger.info("Outlet terminated with reason: #{inspect(reason)}, disconnecting")

    :ok =
      Worker.route(
        %Message{payload: TunnelProtocol.encode(:disconnect), onward_route: peer},
        state
      )
  end

  def terminate(reason, _state) do
    Logger.info("Outlet terminated with reason: #{inspect(reason)}, already disconnected")
    :ok
  end

  defp handle_protocol_msg(state, :disconnect),
    do: {:stop, :normal, Map.put(state, :connected, false)}

  defp handle_protocol_msg(
         %{protocol: %{send_mod: send_mod}, socket: socket, tcp_wrapper: tcp_wrapper} = state,
         {:payload, data}
       ) do
    :ok = tcp_wrapper.wrap_tcp_call(send_mod, :send, [socket, data])

    {:ok, state}
  end

  defp maybe_upgrade_to_ssl(socket, false, _ssl_options, _timeout) do
    {:ok, socket}
  end

  defp maybe_upgrade_to_ssl(socket, true, ssl_options, timeout) do
    ## TODO: insert_server_name_indication??
    :ssl.connect(socket, ssl_options, timeout)
  end
end
