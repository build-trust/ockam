defmodule Ockam.Transport.TCP.Handler.Tests do
  use ExUnit.Case

  alias Ockam.Message
  alias Ockam.Node
  alias Ockam.Transport.TCP

  test "TCP closes idle connections" do
    assert {:ok, _listener_address_b} =
             TCP.start(listen: [port: 6001, handler_options: [idle_timeout: 100]])

    {:ok, client_addr} = TCP.Client.create(host: "localhost", port: 6001)

    {:ok, test_addr} = Node.register_random_address()

    on_exit(fn ->
      Ockam.Node.stop(client_addr)
    end)

    # Check that connection is working
    request = %{
      onward_route: [
        client_addr,
        test_addr
      ],
      return_route: [],
      payload: "hello"
    }

    Ockam.Router.route(request)
    assert_receive(%Message{payload: "hello"})

    # After a period of inactivity, connection must be terminated by the receiving end
    ref = Process.monitor(Node.whereis(client_addr))
    assert_receive({:DOWN, ^ref, :process, _pid, :normal}, 1000)
  end
end
