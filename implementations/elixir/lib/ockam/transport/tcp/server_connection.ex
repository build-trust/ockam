defmodule Ockam.Transport.TCP.Server.Connection do
  @moduledoc false

  @behaviour :ranch_protocol

  use GenServer

  def start_link(ref, transport, opts) do
    pid = :proc_lib.spawn_link(__MODULE__, :init, [{ref, transport, opts}])
    {:ok, pid}
  end

  def init({ref, transport, _opts}) do
    IO.puts("Starting protocol")

    {:ok, socket} = :ranch.handshake(ref)
    :ok = transport.setopts(socket, [{:active, true}])
    :gen_server.enter_loop(__MODULE__, [], %{socket: socket, transport: transport})
  end

  def handle_info({:tcp, socket, data}, %{socket: socket, transport: transport} = state) do
    IO.puts("#{inspect(data)}")
    transport.send(socket, data)
    {:noreply, state}
  end

  def handle_info({:tcp_closed, socket}, %{socket: socket, transport: transport} = state) do
    IO.puts("Closing")
    transport.close(socket)
    {:stop, :normal, state}
  end
end
