defmodule Ockam.Transport.Portal.OutletWorker do
  @moduledoc """
  Portal protocol, Outlet worker
  """

  use Ockam.Worker
  alias Ockam.Message
  alias Ockam.Router
  alias Ockam.Transport.Portal.TunnelProtocol

  require Logger

  @impl true
  def setup(options, state) do
    msg = options[:init_message]
    target_host = options[:target_host]
    target_port = options[:target_port]

    Logger.info(
      "Starting outlet worker to #{target_host}:#{target_port}.  peer: #{inspect(msg.return_route)}"
    )

    with {:ok, :ping} <- TunnelProtocol.decode(msg.payload),
         {:ok, socket} <- :gen_tcp.connect(target_host, target_port, [{:active, :once}, :binary]) do
      :ok = Router.route(Message.reply(msg, state.address, TunnelProtocol.encode(:pong)))
      {:ok, state |> Map.put(:socket, socket) |> Map.put(:peer, msg.return_route)}
    else
      error ->
        Logger.error("Error starting outlet: #{inspect(options)} : #{inspect(error)}")
        :ok = Router.route(Message.reply(msg, state.address, TunnelProtocol.encode(:disconnect)))
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
  def handle_info({:tcp, socket, data}, %{peer: peer} = state) do
    :ok =
      Router.route(%Message{payload: TunnelProtocol.encode({:payload, data}), onward_route: peer})

    :inet.setopts(socket, active: :once)
    {:noreply, state}
  end

  def handle_info({:tcp_closed, _socket}, %{peer: peer} = state) do
    Logger.info("Socket closed")
    :ok = Router.route(%Message{payload: TunnelProtocol.encode(:disconnect), onward_route: peer})
    {:stop, :normal, state}
  end

  def handle_info({:tcp_error, _socket, reason}, %{peer: peer} = state) do
    Logger.info("Socket error: #{inspect(reason)}")
    :ok = Router.route(%Message{payload: TunnelProtocol.encode(:disconnect), onward_route: peer})
    {:stop, {:error, reason}, state}
  end

  def handle_protocol_msg(state, :disconnect), do: {:stop, :normal, state}

  def handle_protocol_msg(state, {:payload, data}) do
    :ok = :gen_tcp.send(state.socket, data)
    {:ok, state}
  end
end
