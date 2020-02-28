defmodule Ockam.Transport.TCP.Listener do
  use GenStateMachine, callback_mode: :state_functions

  alias Ockam.Transport.Socket
  alias Ockam.Transport.TCP.ConnectionSupervisor

  require Logger

  def child_spec([sup, args]) do
    %{
      id: __MODULE__,
      start: {__MODULE__, :start_link, [sup, args]},
      restart: :transient,
      shutdown: 1_000,
      type: :worker
    }
  end

  defmodule State do
    defstruct [:socket, :connection_supervisor, :select_info, :config]
  end

  def start_link(sup, config) do
    GenStateMachine.start_link(__MODULE__, [sup, config])
  end

  def init([sup, config]) do
    socket =
      case :socket.open(:inet, :stream, :tcp) do
        {:ok, socket} ->
          socket

        {:error, _reason} = err ->
          exit(err)
      end

    with :ok <- :socket.setopt(socket, :socket, :keepalive, true),
         :ok <- :socket.setopt(socket, :socket, :reuseaddr, true),
         {:ok, _p} <- :socket.bind(socket, config.listen_address),
         :ok <- :socket.listen(socket) do
      state = %State{socket: socket, connection_supervisor: sup, select_info: nil, config: config}

      {:ok, :accept, state, [{:next_event, :internal, :accept}]}
    else
      {:error, _reason} = err ->
        :socket.close(socket)
        err
    end
  end

  def accept(:internal, :accept, %State{socket: socket, config: config} = state) do
    case :socket.accept(socket, :nowait) do
      {:ok, conn} ->
        sock = Socket.new(:server, conn, config.listen_address)

        case ConnectionSupervisor.new_connection(state.connection_supervisor, sock) do
          {:ok, pid} ->
            :ok = :socket.setopt(conn, :otp, :controlling_process, pid)
            :ok = GenStateMachine.call(pid, {:socket_controller, conn})
            {:keep_state_and_data, [{:next_event, :internal, :accept}]}
        end

      {:select, {:select_info, :accept, info}} ->
        {:keep_state, %State{state | select_info: info}}

      {:error, reason} ->
        :socket.close(socket)
        {:stop, reason, %State{state | socket: nil}}
    end
  end

  def accept(:info, {:"$socket", _socket, :select, info}, %State{select_info: info} = data) do
    accept(:internal, :accept, %State{data | select_info: nil})
  end

  def terminate(_reason, _state, %State{socket: nil}), do: :ok

  def terminate(_reason, _state, %State{socket: socket}) do
    :socket.close(socket)
    :ok
  end
end
