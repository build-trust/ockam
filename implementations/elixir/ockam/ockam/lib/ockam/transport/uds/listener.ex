defmodule Ockam.Transport.UDS.Listener do
  use Task, restart: :transient

  alias Ockam.Transport.UDS.ConnectionSupervisor

  require Logger

  @default_socket "/tmp/ockam_uds_server.sock"

  def start_link(opts \\ []) do
    Task.start_link(__MODULE__, :run, [Keyword.get(opts, :path, @default_socket)])
  end

  def run(path) do
    case :gen_tcp.listen(0, [
           :binary,
           {:active, true},
           {:ifaddr, {:local, path}}
         ]) do
      {:ok, listen_socket} ->
        Logger.info("UDS server listening on socket: #{inspect(path)}")

        accept_loop(listen_socket)

      {:error, reason} ->
        raise "failed to listen on socket #{inspect(path)}: #{inspect(reason)}"
    end
  end

  def accept_loop(listen_socket) do
    case :gen_tcp.accept(listen_socket) do
      {:ok, socket} ->
        ConnectionSupervisor.start_child(socket)
        accept_loop(listen_socket)

      {:error, reason} ->
        raise "failed to accept connection: #{inspect(reason)}"
    end
  end
end
