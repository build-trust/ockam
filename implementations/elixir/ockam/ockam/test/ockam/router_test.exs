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
          return_route: [address],
          payload: Message.payload(message)
        }

        Logger.info("\nMESSAGE: #{inspect(message)}\nFORWARD: #{inspect(forward)}")
        Router.route(forward)

        {:ok, state}
    end
  end
end

defmodule Ockam.Router.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Router

  alias Ockam.Router.Tests.Echo
  alias Ockam.Router.Tests.Forwarder
  alias Ockam.Router.Tests.Printer
  alias Ockam.Transport.TCP
  alias Ockam.Transport.TCPAddress
  alias Ockam.Transport.UDP
  alias Ockam.Transport.UDPAddress

  setup_all do
    {:ok, "printer"} = Printer.create(address: "printer")
    {:ok, "echo"} = Echo.create(address: "echo")
    {:ok, "client_forwarder"} = Forwarder.create(address: "client_forwarder")
    printer_pid = Ockam.Node.whereis("printer")
    echo_pid = Ockam.Node.whereis("echo")
    forwarder_pid = Ockam.Node.whereis("client_forwarder")
    [printer_pid: printer_pid, echo_pid: echo_pid, forwarder_pid: forwarder_pid]
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
          %TCPAddress{ip: {127, 0, 0, 1}, port: 4000},
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

    test "TCP echo test", %{echo_pid: echo, forwarder_pid: client_forwarder} do
      # client
      request = %{
        onward_route: [
          "client_forwarder",
          %TCPAddress{ip: {127, 0, 0, 1}, port: 5000},
          "echo"
        ],
        return_route: [],
        payload: "hello"
      }

      :erlang.trace(echo, true, [:receive])
      :erlang.trace(client_forwarder, true, [:receive])

      assert {:ok, _address_a} = TCP.create_listener(port: 6000, route_outgoing: true)

      assert {:ok, _address_b} = TCP.create_listener(port: 5000)

      Ockam.Router.route(request)

      assert_receive(
        {
          :trace,
          ^client_forwarder,
          :receive,
          %{
            onward_route: [
              "client_forwarder",
              %TCPAddress{ip: {127, 0, 0, 1}, port: 5000},
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
           return_route: [registered_tcp_thing, "client_forwarder"],
           payload: _
         }},
        1_000
      )

      tcp_pid = Ockam.Node.whereis(registered_tcp_thing)

      assert is_pid(tcp_pid)

      :erlang.trace(tcp_pid, true, [:receive])

      # echo service response
      assert_receive(
        {:trace, ^tcp_pid, :receive,
         %{
           onward_route: [^registered_tcp_thing, "client_forwarder"],
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
            return_route: [%TCPAddress{ip: {127, 0, 0, 1}, port: 5000}, "echo"]
          }
        },
        1_000
      )
    end
  end
end
