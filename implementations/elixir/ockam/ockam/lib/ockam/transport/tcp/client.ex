defmodule Ockam.Transport.TCP.Client do
  @moduledoc false
  use GenServer

  @impl true
  def init(%{ip: ip, port: port} = state) do
    # TODO: connect/3 and controlling_process/2 should be in a callback.
    {:ok, socket} = :gen_tcp.connect(ip, port, [:binary, :inet, active: true, packet: 2])
    :gen_tcp.controlling_process(socket, self())

    {:ok, Map.put(state, :socket, socket)}
  end

  @spec start_link(map) :: :ignore | {:error, any} | {:ok, pid}
  def start_link(default) when is_map(default) do
    GenServer.start_link(__MODULE__, default)
  end

  @impl true
  def handle_info(:connect, %{ip: ip, port: port} = state) do
    {:ok, socket} = :gen_tcp.connect(ip, port, [:binary, :inet, {:active, true}, {:packet, 2}])
    :inet.setopts(socket, [{:packet, 2}])
    :gen_tcp.controlling_process(socket, self())

    {:noreply, Map.put(state, :socket, socket)}
  end

  def handle_info({:tcp, _socket, _data}, state) do
    {:noreply, state}
  end

  def handle_info({:tcp_closed, _}, state), do: {:stop, :normal, state}
  def handle_info({:tcp_error, _}, state), do: {:stop, :normal, state}

  @impl true
  def handle_call({:send, data}, _from, %{socket: socket} = state) do
    {:reply, :gen_tcp.send(socket, data), state}
  end

  @spec send(atom | pid | {atom, any} | {:via, atom, any}, any) :: any
  def send(pid, data) do
    GenServer.call(pid, {:send, data})
  end
end
