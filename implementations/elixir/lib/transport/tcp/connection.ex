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
      Socket.close(data.socket)
      {:stop, {type, reason}, %State{data | socket: nil}}
  end

  def handshake(:internal, :in, %State{socket: socket, handshake: hs} = data) do
    case Transport.recv(socket) do
      {:ok, bindata, socket} ->
        case Handshake.read_message(hs, bindata) do
          {:ok, hs, _payload} ->
            case Handshake.next_message(hs) do
              :done ->
                {:ok, chan} = Handshake.finalize(hs)

                {:next_state, :connected,
                 %State{data | socket: socket, handshake: nil, channel: chan, next: :done},
                 [{:next_event, :internal, :recv}]}

              next ->
                {:keep_state, %State{data | socket: socket, handshake: hs, next: next},
                 [{:next_event, :internal, next}]}
            end

          {:error, reason} ->
            {:ok, _socket} = Transport.close(socket)
            {:stop, reason, %State{data | socket: nil}}
        end

      {:select, {:select_info, :recv, info}} ->
        {:keep_state, %State{data | select_info: info, next: :in}}

      {:error, reason} ->
        {:ok, _socket} = Transport.close(socket)
        {:stop, reason, %State{data | socket: nil}}
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
                 [{:next_event, :internal, :recv}]}

              next ->
                {:keep_state, %State{data | socket: socket, handshake: hs, next: next},
                 [{:next_event, :internal, next}]}
            end

          {:error, reason} ->
            {:ok, _socket} = Transport.close(socket)
            {:stop, reason, %State{data | socket: nil}}
        end

      {:error, reason} ->
        {:ok, _socket} = Transport.close(socket)
        {:stop, reason, %State{data | socket: nil}}
    end
  end

  def handshake(
        :info,
        {:"$socket", _socket, :select, info},
        %State{select_info: info, next: next} = data
      ) do
    handshake(:internal, next, %State{data | select_info: nil})
  end

  def handshake(
        :info,
        {:"$socket", _socket, :abort, {info, reason}},
        %State{select_info: info} = data
      ) do
    {:ok, _} = Transport.close(data.socket)
    {:stop, reason, %State{data | socket: nil}}
  end

  def connected(:internal, :receive, %State{socket: socket, data: prev} = data) do
    case Socket.recv(socket) do
      {:ok, bin} ->
        received = prev <> bin

        case Transport.decode(received) do
          {:ok, message, rest} ->
            decrypt_and_handle_message(message, %State{data | data: rest})

          {:more, _} ->
            new_data = %State{data | data: bin}
            {:keep_state, new_data, [{:next_event, :internal, :receive}]}

          {:error, reason} ->
            {:ok, _} = Socket.close(socket)
            {:stop, reason, %State{data | socket: nil}}
        end

      {:select, {:select_info, :select, info}} ->
        {:keep_state, %State{data | select_info: info}}

      {:error, reason} ->
        {:ok, _} = Socket.close(socket)
        {:stop, reason, %State{data | socket: nil}}
    end
  end

  def connected(:info, {:"$socket", _socket, :select, info}, %State{select_info: info} = data) do
    connected(:internal, :receive, %State{data | select_info: nil})
  end

  def connected(
        :info,
        {:"$socket", _socket, :abort, {info, reason}},
        %State{select_info: info} = data
      ) do
    Socket.close(data.socket)
    {:stop, reason, %State{data | socket: nil}}
  end

  defp decrypt_and_handle_message(message, %State{channel: chan, socket: socket} = state) do
    case Channel.decrypt(chan, message) do
      {:ok, new_chan, decrypted} ->
        handle_message(decrypted, %State{state | channel: new_chan})

      {:error, reason} ->
        Socket.close(socket)
        {:stop, reason, %State{state | socket: nil}}
    end
  end

  defp handle_message("ACK", %State{} = _state) do
    Logger.info("Connection established and secured successfully!")
    :keep_state_and_data
  end

  defp handle_message(msg, _data) do
    {:stop, {:invalid_data, msg}}
  end

  def terminate(_reason, _state, %State{socket: nil}), do: :ok

  def terminate(_reason, _state, %State{socket: socket}) do
    Socket.close(socket)
    :ok
  end
end
