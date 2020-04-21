defmodule Ockam.Transport.TCP.Connection do
  use GenStateMachine, callback_mode: :state_functions

  require Logger

  alias Ockam.Transport
  alias Ockam.Transport.Socket
  alias Ockam.Channel
  alias Ockam.Channel.Handshake
  alias Ockam.Vault.KeyPair

  @protocol "Noise_XX_25519_AESGCM_SHA256"

  def child_spec(args) do
    %{
      id: __MODULE__,
      start: {__MODULE__, :start_link, [args]},
      restart: :temporary,
      shutdown: 1_000,
      type: :worker
    }
  end

  defmodule State do
    defstruct [:socket, :data, :mode, :select_info, :handshake, :channel, :next]
  end

  def start_link(_opts, socket) do
    GenStateMachine.start_link(__MODULE__, [socket])
  end

  def init([socket]) do
    data = %State{socket: socket, data: "", select_info: nil}
    {:ok, :initializing, data}
  end

  def initializing({:call, from}, {:socket_controller, _conn}, data) do
    GenStateMachine.reply(from, :ok)
    e = KeyPair.new(:x25519)
    s = KeyPair.new(:x25519)

    {:ok, handshake} = Channel.handshake(:responder, %{protocol: @protocol, e: e, s: s})

    next = Handshake.next_message(handshake)
    new_data = %State{data | handshake: handshake, next: next}
    {:next_state, :handshake, new_data, [{:next_event, :internal, next}]}
  catch
    type, reason ->
      stop(data, {type, reason})
  end

  def handshake(:internal, :in, %State{socket: socket, handshake: hs} = data) do
    case Transport.recv_nonblocking(socket) do
      {:ok, bindata, socket} ->
        case Handshake.read_message(hs, bindata) do
          {:ok, hs, _payload} ->
            case Handshake.next_message(hs) do
              :done ->
                {:ok, chan} = Handshake.finalize(hs)

                {:next_state, :connected,
                 %State{data | socket: socket, handshake: nil, channel: chan, next: :done},
                 [{:next_event, :internal, :ack}]}

              next ->
                {:keep_state, %State{data | socket: socket, handshake: hs, next: next},
                 [{:next_event, :internal, next}]}
            end

          {:error, reason} ->
            stop(data, reason)
        end

      {:wait, {:recv, info}, socket} ->
        {:keep_state, %State{data | socket: socket, select_info: info, next: :in}}

      {:error, reason} ->
        stop(data, reason)
    end
  end

  def handshake(:internal, :out, %State{socket: socket, handshake: hs} = data) do
    case Handshake.write_message(hs, "") do
      {:ok, hs, msg} ->
        case Transport.send(socket, msg) do
          {:ok, socket} ->
            case Handshake.next_message(hs) do
              :done ->
                {:ok, chan} = Handshake.finalize(hs)

                {:next_state, :connected,
                 %State{data | socket: socket, handshake: nil, channel: chan, next: :done},
                 [{:next_event, :internal, :ack}]}

              next ->
                {:keep_state, %State{data | socket: socket, handshake: hs, next: next},
                 [{:next_event, :internal, next}]}
            end

          {:error, reason} ->
            stop(data, reason)
        end

      {:error, reason} ->
        stop(data, reason)
    end
  end

  def handshake(
        :info,
        {:"$socket", _socket, :select, info} = msg,
        %State{select_info: info, next: next} = data
      ) do
    {:ok, :recv, new_socket} = Socket.handle_message(data.socket, msg)
    handshake(:internal, next, %State{data | socket: new_socket, select_info: nil})
  end

  def handshake(
        :info,
        {:"$socket", _socket, :abort, {info, _reason}} = msg,
        %State{select_info: info} = data
      ) do
    {:error, {:abort, reason}, new_socket} = Socket.handle_message(data.socket, msg)
    stop(%State{data | socket: new_socket, select_info: nil}, reason)
  end

  def connected(:internal, :ack, %State{channel: chan, socket: socket} = data) do
    Logger.debug("Connection established, sending ACK..")
    # Send ACK then start receiving
    with {:ok, new_chan, encrypted} <- Channel.encrypt(chan, "ACK"),
         {:ok, new_socket} <- Socket.send(socket, encrypted) do
      new_data = %State{data | channel: new_chan, socket: new_socket}
      {:keep_state, new_data, [{:next_event, :internal, :receive}]}
    else
      {:error, reason} = err ->
        stop(data, reason)
    end
  end

  def connected(:internal, :receive, %State{socket: socket} = data) do
    Logger.debug("Entering receive state..")

    case Socket.recv_nonblocking(socket) do
      {:ok, message, new_socket} ->
        decrypt_and_handle_message(message, %State{data | socket: new_socket})

      {:wait, {:recv, info}, new_socket} ->
        {:keep_state, %State{data | socket: new_socket, select_info: info}}

      {:error, reason} ->
        stop(data, reason)
    end
  end

  def connected(
        :info,
        {:"$socket", _socket, :select, info} = msg,
        %State{select_info: info} = data
      ) do
    {:ok, :recv, new_socket} = Socket.handle_message(data.socket, msg)
    connected(:internal, :receive, %State{data | socket: new_socket, select_info: nil})
  end

  def connected(
        :info,
        {:"$socket", _socket, :abort, {info, _reason}} = msg,
        %State{select_info: info} = data
      ) do
    {:error, {:abort, reason}, new_socket} = Socket.handle_message(data.socket, msg)
    stop(%State{data | socket: new_socket, select_info: nil}, reason)
  end

  def connect(type, msg, state) do
    Logger.warn("Unexpected message received in :connect state: #{inspect({type, msg})}")
    {:keep_state, state, [{:next_event, :internal, :receive}]}
  end

  defp decrypt_and_handle_message(message, %State{channel: chan} = data) do
    case Channel.decrypt(chan, message) do
      {:ok, new_chan, decrypted} ->
        handle_message(decrypted, %State{data | channel: new_chan})

      {:error, reason} ->
        stop(data, reason)
    end
  end

  defp handle_message("OK", %State{} = data) do
    Logger.debug("ACK was received and acknowledged successfully!")
    {:keep_state, data, [{:next_event, :internal, :receive}]}
  end

  defp handle_message(msg, data) do
    stop(data, {:unknown_message, msg})
  end

  defp stop(%State{socket: nil} = data, :closed) do
    {:stop, :shutdown, data}
  end

  defp stop(%State{socket: nil} = data, reason) do
    {:stop, reason, data}
  end

  defp stop(%State{socket: socket} = data, reason) do
    case Transport.close(socket) do
      {:ok, new_socket} ->
        {:stop, reason, %State{data | socket: new_socket}}

      {:error, :closed} when reason == :closed ->
        # Already closed
        {:stop, :shutdown, %State{data | socket: nil}}

      {:error, :closed} ->
        # Already closed
        {:stop, reason, %State{data | socket: nil}}

      {:error, err} ->
        Logger.warn("Failed to cleanly shutdown socket: #{inspect(err)}")
        {:stop, reason, %State{data | socket: nil}}
    end
  end
end
