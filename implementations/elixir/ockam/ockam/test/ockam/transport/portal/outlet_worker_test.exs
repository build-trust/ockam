defmodule Ockam.Transport.Portal.OutletWorker.Tests do
  use ExUnit.Case
  doctest Ockam.Transport.Portal.OutletWorker
  alias Ockam.Message
  alias Ockam.Transport.Portal.OutletWorker
  alias Ockam.Transport.Portal.TunnelProtocol

  @sample_data "0123456789"

  describe "Ockam.Transport.Portal.OutletWorker" do
    test "data is tunneled through the worker" do
      {:ok, echo_port} = spawn_tcp_echo()
      {:ok, me} = Ockam.Node.register_random_address()

      # Create the worker, as if created through a spawner
      ping = %Message{
        payload: TunnelProtocol.encode(:ping),
        onward_route: [],
        return_route: [me]
      }

      {:ok, worker} =
        OutletWorker.create([
          {:init_message, ping},
          {:target_host, 'localhost'},
          {:target_port, echo_port}
        ])

      # We must receive the :pong response handshake
      assert :pong == receive_msg()

      # Send some data
      data = %Message{
        payload: TunnelProtocol.encode({:payload, {@sample_data, 0}}),
        onward_route: [worker],
        return_route: [me]
      }

      :ok = Ockam.Router.route(data)

      # The echo listener echo with the same data, we must receive it back
      assert {:payload, {@sample_data, 0}} == receive_msg()

      # Echo listener closes the socket, we must receive the :disconnect message
      assert :disconnect == receive_msg()
    end

    test "wrong packet counter, connection closed" do
      {:ok, echo_port} = spawn_tcp_echo()
      {:ok, me} = Ockam.Node.register_random_address()

      # Create the worker, as if created through a spawner
      ping = %Message{
        payload: TunnelProtocol.encode(:ping),
        onward_route: [],
        return_route: [me]
      }

      {:ok, worker} =
        OutletWorker.create([
          {:init_message, ping},
          {:target_host, 'localhost'},
          {:target_port, echo_port}
        ])

      # We must receive the :pong response handshake
      assert :pong == receive_msg()

      # Send some data with wrong packet counter (should be 0)
      data = %Message{
        payload: TunnelProtocol.encode({:payload, {@sample_data, 1}}),
        onward_route: [worker],
        return_route: [me]
      }

      :ok = Ockam.Router.route(data)

      # packet counter check failed, connection closed
      assert :disconnect == receive_msg()
    end
  end

  # Spawns a simple tcp listener that waits till receiving byte_size(@sample_data) bytes,
  # echoes that data back, and closes the socket.
  defp spawn_tcp_echo() do
    ref = make_ref()
    parent = self()
    spawn_link(fn -> tcp_echo(parent, ref) end)

    receive do
      {^ref, port} -> {:ok, port}
    after
      60_000 ->
        {:error, "Can't listen on socket"}
    end
  end

  defp tcp_echo(parent, ref) do
    {:ok, listen_socket} = :gen_tcp.listen(0, [{:active, false}, {:packet, :raw}, :binary])
    {:ok, port} = :inet.port(listen_socket)
    send(parent, {ref, port})
    {:ok, socket} = :gen_tcp.accept(listen_socket)
    {:ok, data} = :gen_tcp.recv(socket, byte_size(@sample_data), 60_000)
    :ok = :gen_tcp.send(socket, data)
    :gen_tcp.close(socket)
  end

  defp receive_msg() do
    receive do
      %Ockam.Message{payload: data} ->
        {:ok, msg} = TunnelProtocol.decode(data)
        msg
    after
      60_000 ->
        raise "Timeout waiting for msg"
    end
  end
end
