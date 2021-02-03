defmodule Ockam.Transport.TCP.Client.GenTcp do
  @moduledoc false

  use Ockam.Worker

  @doc false
  @impl true
  def setup(options, state) do
    ip = Keyword.get_lazy(options, :ip, &default_ip/0)
    port = Keyword.get_lazy(options, :port, &default_port/0)
    tcp_connect_options = [:binary, active: :once]

    state = Map.put(state, :ip, ip)
    state = Map.put(state, :port, port)
    state = Map.put(state, :tcp_connect_options, tcp_connect_options)

    case :gen_tcp.connect(ip, port, tcp_connect_options) do
      {:ok, socket} ->
        state = Map.put(state, :socket, socket)
        {:ok, state}

      {:error, reason} ->
        {:error, {:could_not_tcp_connect, reason, state}}
    end
  end

  @doc false
  @impl true
  def handle_message({:tcp, _, _data} = message, state) do
    IO.puts("#{inspect(message)}")
    {:noreply, state}
  end

  def handle_message({:tcp_closed, _}, state), do: {:stop, :normal, state}
  def handle_message({:tcp_error, _}, state), do: {:stop, :normal, state}

  def handle_message(message, %{socket: socket} = state) do
    :ok = :gen_tcp.send(socket, message)
    {:noreply, state}
  end

  defp default_ip, do: {127, 0, 0, 1}
  defp default_port, do: 4000
end
