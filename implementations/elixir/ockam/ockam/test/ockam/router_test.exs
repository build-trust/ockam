defmodule Ockam.Router.Tests.Printer do
  # this has to be outside the test module for the macro to work
  # because the macro uses defp.
  use Ockam.Worker

  @impl true
  def handle_message(_message, state) do
    {:ok, state}
  end
end

defmodule Ockam.Router.Tests do
  use ExUnit.Case, async: true
  doctest Ockam.Router

  alias Ockam.Router.Tests.Printer
  alias Ockam.Transport.TCP
  alias Ockam.Transport.TCPAddress
  alias Ockam.Transport.UDP
  alias Ockam.Transport.UDPAddress

  setup_all do
    {:ok, "printer"} = Printer.create(address: "printer")
    printer_pid = Ockam.Node.whereis("printer")
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
  end
end
