defmodule Ockam.Router.Tests.Printer do
  # this has to be outside the test module for the macro to work
  # because the macro uses defp.
  use Ockam.Worker

  @impl true
  def handle_message(_message, state) do
    {:ok, state}
  end
end

defmodule Ockam.Router.Tests.Echo do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    reply = %{
      onward_route: Message.return_route(message),
      return_route: [state.address],
      payload: Message.payload(message)
    }

    Logger.info("\nMESSAGE: #{inspect(message)}\nREPLY: #{inspect(reply)}")
    Router.route(reply)

    {:ok, state}
  end
end

defmodule Ockam.Router.Tests.Forwarder do
  # this has to be outside the test module for the macro to work
  # because the macro uses defp.
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    address = state.address

    case Message.onward_route(message) do
      [^address] ->
        Logger.info("\nMESSAGE BACK: #{inspect(message)}}")
        {:ok, state}

      [^address | rest] ->
        forward = %{
          onward_route: rest,
          return_route: [address | Message.return_route(message)],
          payload: Message.payload(message)
        }

        Logger.info("\nMESSAGE: #{inspect(message)}\nFORWARD: #{inspect(forward)}")
        Router.route(forward)

        {:ok, state}
    end
  end
end

defmodule Ockam.Router.Tests.PingPong do
  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router

  require Logger

  @impl true
  def handle_message(message, state) do
    payload = Message.payload(message)

    response_payload =
      case payload do
        "ping " <> n ->
          "pong " <> n

        "pong " <> n ->
          next = next(n)
          "ping " <> next
      end

    reply = %{
      onward_route: Message.return_route(message),
      return_route: [state.address],
      payload: response_payload
    }

    Logger.info("\nMESSAGE: #{inspect(message)}\nREPLY: #{inspect(reply)}")
    Router.route(reply)

    {:ok, state}
  end

  def next(n) do
    int_n = n |> String.trim() |> String.to_integer()
    to_string(int_n + 1)
  end
end

defmodule Ockam.Router.Tests do
  use ExUnit.Case, async: false
  doctest Ockam.Router

  alias Ockam.Router.Tests.Echo
  alias Ockam.Router.Tests.Forwarder
  alias Ockam.Router.Tests.PingPong
  alias Ockam.Router.Tests.Printer

  alias Ockam.Transport.TCP
  alias Ockam.Transport.TCPAddress
  alias Ockam.Transport.UDP
  alias Ockam.Transport.UDPAddress

  require Logger

  setup do
    {:ok, "printer"} = Printer.create(address: "printer")
    printer_pid = Ockam.Node.whereis("printer")

    on_exit(fn ->
      Ockam.Node.stop("printer")
    end)

    [printer_pid: printer_pid]
  end

  describe "Ockam.Router" do
    test "Simple UDP Test", %{printer_pid: printer} do
      message = %{
        onward_route: [
          %UDPAddress{ip: {127, 0, 0, 1}, port: 4000},
          "printer"
        ],
        payload: "hello"
      }

      :erlang.trace(printer, true, [:receive])

      assert {:ok, _address_a} = UDP.create_listener(port: 3000, route_outgoing: true)

      assert {:ok, _address_b} = UDP.create_listener(port: 4000)

      Ockam.Router.route(message)

      assert_receive({:trace, ^printer, :receive, result}, 1_000)

      assert result == %{
               version: 1,
               onward_route: ["printer"],
               payload: "hello",
               return_route: [
                 %UDPAddress{ip: {127, 0, 0, 1}, port: 3000}
               ]
             }
    end

    test "Simple TCP Test", %{printer_pid: printer} do
      message = %{
        onward_route: [
          %TCPAddress{host: {127, 0, 0, 1}, port: 4000},
          "printer"
        ],
        return_route: [],
        payload: "hello"
      }

      :erlang.trace(printer, true, [:receive])

      assert {:ok, _address_a} = TCP.create_listener(port: 3000, route_outgoing: true)

      assert {:ok, _address_b} = TCP.create_listener(port: 4000)

      Ockam.Router.route(message)

      assert_receive({:trace, ^printer, :receive, result}, 1_000)

      assert %{
               version: 1,
               onward_route: ["printer"],
               payload: "hello",
               return_route: [_address]
             } = result
    end

    test "TCP echo test" do
      {:ok, "echo"} = Echo.create(address: "echo")
      {:ok, "client_forwarder"} = Forwarder.create(address: "client_forwarder")
      echo = Ockam.Node.whereis("echo")
      client_forwarder = Ockam.Node.whereis("client_forwarder")

      on_exit(fn ->
        Ockam.Node.stop("echo")
        Ockam.Node.stop("client_forwarder")
      end)

      # client
      request = %{
        onward_route: [
          "client_forwarder",
          %TCPAddress{host: {127, 0, 0, 1}, port: 5000},
          "echo"
        ],
        return_route: [],
        payload: "hello"
      }

      :erlang.trace(echo, true, [:receive])
      :erlang.trace(client_forwarder, true, [:receive])

      assert {:ok, _listener_address_a} = TCP.create_listener(port: 6000, route_outgoing: true)

      assert {:ok, _listener_address_b} = TCP.create_listener(port: 5000)

      Ockam.Router.route(request)

      assert_receive(
        {
          :trace,
          ^client_forwarder,
          :receive,
          %{
            onward_route: [
              "client_forwarder",
              %TCPAddress{host: {127, 0, 0, 1}, port: 5000},
              "echo"
            ],
            return_route: []
          }
        },
        1_000
      )

      # tcp sends to echo on hub
      assert_receive(
        {:trace, ^echo, :receive,
         %{
           onward_route: [
             "echo"
           ],
           return_route: [registered_server_tcp, "client_forwarder"],
           payload: _
         }},
        1_000
      )

      tcp_pid = Ockam.Node.whereis(registered_server_tcp)

      assert is_pid(tcp_pid)

      :erlang.trace(tcp_pid, true, [:receive])

      # echo service response
      assert_receive(
        {:trace, ^tcp_pid, :receive,
         %{
           onward_route: [^registered_server_tcp, "client_forwarder"],
           return_route: ["echo"]
         }},
        1_000
      )

      assert_receive(
        {
          :trace,
          ^client_forwarder,
          :receive,
          %{
            onward_route: ["client_forwarder"],
            return_route: [registered_client_tcp, "echo"]
          }
        },
        1_000
      )

      client_tcp_pid = Ockam.Node.whereis(registered_client_tcp)

      assert is_pid(client_tcp_pid)
    end

    test "TCP ping pong test" do
      {:ok, "ping_pong_server"} = PingPong.create(address: "ping_pong_server")
      {:ok, "ping_pong_client"} = PingPong.create(address: "ping_pong_client")
      ping_pong_server = Ockam.Node.whereis("ping_pong_server")
      ping_pong_client = Ockam.Node.whereis("ping_pong_client")

      :erlang.trace(ping_pong_server, true, [:receive])
      :erlang.trace(ping_pong_client, true, [:receive])

      on_exit(fn ->
        Ockam.Node.stop("ping_pong_server")
        Ockam.Node.stop("ping_pong_client")
      end)

      ## Initial request
      request = %{
        onward_route: [
          %TCPAddress{host: {127, 0, 0, 1}, port: 5001},
          "ping_pong_server"
        ],
        return_route: [
          "ping_pong_client"
        ],
        payload: "ping 1"
      }

      assert {:ok, _listener_address_a} = TCP.create_listener(port: 6001, route_outgoing: true)

      assert {:ok, _listener_address_b} = TCP.create_listener(port: 5001)

      Ockam.Router.route(request)

      assert_receive(
        {
          :trace,
          ^ping_pong_server,
          :receive,
          %{
            onward_route: [
              "ping_pong_server"
            ],
            return_route: [registered_server_tcp, "ping_pong_client"],
            payload: "ping 1"
          }
        },
        1_000
      )

      assert_receive(
        {
          :trace,
          ^ping_pong_client,
          :receive,
          %{
            onward_route: [
              "ping_pong_client"
            ],
            return_route: [registered_client_tcp, "ping_pong_server"],
            payload: "pong 1"
          }
        },
        1_000
      )

      server_tcp_pid = Ockam.Node.whereis(registered_server_tcp)

      assert is_pid(server_tcp_pid)

      :erlang.trace(server_tcp_pid, true, [:receive])

      client_tcp_pid = Ockam.Node.whereis(registered_client_tcp)

      assert is_pid(client_tcp_pid)

      :erlang.trace(client_tcp_pid, true, [:receive])

      # Server and client TCP workers receive messages to forward over TCP
      assert_receive(
        {:trace, ^server_tcp_pid, :receive,
         %{
           onward_route: [^registered_server_tcp, "ping_pong_client"],
           return_route: ["ping_pong_server"],
           payload: "pong " <> _
         }},
        1_000
      )

      assert_receive(
        {
          :trace,
          ^client_tcp_pid,
          :receive,
          %{
            onward_route: [^registered_client_tcp, "ping_pong_server"],
            return_route: ["ping_pong_client"],
            payload: "ping " <> _
          }
        },
        1_000
      )

      ## Client and server keep exchanging messages
      assert_receive(
        {
          :trace,
          ^ping_pong_server,
          :receive,
          %{
            onward_route: [
              "ping_pong_server"
            ],
            return_route: [^registered_server_tcp, "ping_pong_client"],
            payload: "ping 10"
          }
        },
        1_000
      )

      assert_receive(
        {
          :trace,
          ^ping_pong_client,
          :receive,
          %{
            onward_route: [
              "ping_pong_client"
            ],
            return_route: [^registered_client_tcp, "ping_pong_server"],
            payload: "pong 10"
          }
        },
        1_000
      )
    end
  end
end
