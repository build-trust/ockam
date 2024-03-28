defmodule Ockam.Transport.Portal.InletWorker do
  @moduledoc """
  Inlet TCP Worker.

  It had three states:
      :wait_for_socket : right after starting. It's waiting for the Listener to transfer control of the socket.
      :wait_for_pong :   already sent the initial :ping message, and are waiting for the pong response.
      :connected :       pong has been received, socket is moved into {active, once} mode to start reading from it.

  TODO: Ideally this should be implemented as a GenFsm, but that's not what Ockam.Worker expect.
        It's also a good candidate to use the session framework on Ockam.Session.*
        Given the simplicity, for now it encoded the current 'state' as part of the GenServer' state.
  """

  use Ockam.Worker
  alias Ockam.Message
  alias Ockam.Transport.Portal.TunnelProtocol
  alias Ockam.Worker
  require Logger

  @impl true
  def setup(options, state) do
    peer_route = options[:peer_route]
    tcp_wrapper = Keyword.get(options, :tcp_wrapper, Ockam.Transport.TCP.DefaultWrapper)
    Logger.info("Starting inlet worker.  Outlet peer: #{inspect(peer_route)}")

    {:ok,
     Map.merge(
       state,
       %{
         peer_route: peer_route,
         stage: :wait_for_socket,
         tcp_wrapper: tcp_wrapper,
         incoming_packet_counter: 0xFFFF,
         outgoing_packet_counter: 0xFFFF
       }
     )}
  end

  @impl true
  def handle_cast({:takeover, socket}, %{stage: :wait_for_socket} = state) do
    # The listener process has set us as the socket' controlling process.
    # From this point on, it's safe to set the socket to active mode at any time.
    :ok =
      Worker.route(
        %Message{
          payload: TunnelProtocol.encode(:ping),
          onward_route: state.peer_route,
          return_route: [state.address]
        },
        state
      )

    {:noreply, Map.merge(state, %{stage: :wait_for_pong, socket: socket})}
  end

  @impl true
  def handle_message(%Message{payload: data} = msg, state) do
    with {:ok, protocol_msg} <- TunnelProtocol.decode(data) do
      handle_protocol_msg(state, protocol_msg, msg.return_route)
    end
  end

  @impl true
  def handle_info(
        {:tcp, socket, data},
        %{
          peer_route: peer_route,
          outgoing_packet_counter: packet_counter
        } = state
      ) do
    :ok = :inet.setopts(socket, active: :once)

    packet_counter = increment_packet_counter(packet_counter)

    :ok =
      Worker.route(
        %Message{
          payload: TunnelProtocol.encode({:payload, {data, packet_counter}}),
          onward_route: peer_route
        },
        state
      )

    state = Map.put(state, :outgoing_packet_counter, packet_counter)
    {:noreply, state}
  end

  def handle_info({:tcp_closed, _socket}, %{peer_route: peer_route} = state) do
    Logger.info("Socket closed")

    :ok =
      Worker.route(
        %Message{
          payload: TunnelProtocol.encode(:disconnect),
          onward_route: peer_route
        },
        state
      )

    {:stop, :normal, state}
  end

  def handle_info({:tcp_error, _socket, reason}, %{peer: peer} = state) do
    Logger.info("Socket error: #{inspect(reason)}")

    :ok =
      Worker.route(
        %Message{payload: TunnelProtocol.encode(:disconnect), onward_route: peer},
        state
      )

    {:stop, {:error, reason}, state}
  end

  defp handle_protocol_msg(%{stage: :wait_for_pong} = state, :pong, return_route) do
    Logger.info("Successful handshake, initiating data transfer")
    :ok = :inet.setopts(state.socket, active: :once)
    {:ok, %{state | stage: :connected, peer_route: return_route}}
  end

  defp handle_protocol_msg(
         %{
           socket: socket,
           stage: :connected,
           tcp_wrapper: tcp_wrapper,
           incoming_packet_counter: packet_counter
         } = state,
         {:payload, {data, msg_packet_counter}},
         _return_route
       ) do
    if msg_packet_counter != :undefined do
      packet_counter = increment_packet_counter(packet_counter)

      if packet_counter != msg_packet_counter do
        Logger.warning(
          "Packet counter mismatch, expected #{packet_counter}, got #{msg_packet_counter}"
        )

        {:stop, :packet_counter_mismatch, state}
      else
        :ok = tcp_wrapper.wrap_tcp_call(:gen_tcp, :send, [socket, data])
        state = Map.put(state, :incoming_packet_counter, packet_counter)
        {:ok, state}
      end
    else
      :ok = tcp_wrapper.wrap_tcp_call(:gen_tcp, :send, [socket, data])
      {:ok, state}
    end
  end

  defp handle_protocol_msg(%{socket: socket} = state, :disconnect, _return_route) do
    Logger.info("Peer disconnected")
    :ok = :gen_tcp.close(socket)
    {:stop, :normal, state}
  end

  defp increment_packet_counter(packet_counter) do
    # 0xFFFF is the maximum value for a u16
    if packet_counter == 0xFFFF do
      0
    else
      packet_counter + 1
    end
  end
end
