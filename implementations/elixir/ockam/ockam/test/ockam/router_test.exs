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
  alias Ockam.Transport.UDPAddress
  alias Ockam.Transport.TCPAddress

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



      assert {:ok, _address_a} =
               Ockam.Transport.UDP.create_listener(port: 3000, route_outgoing: true)
      assert {:ok, _address_b} =
               Ockam.Transport.UDP.create_listener(port: 4000)

      Ockam.Router.route(message)

      assert_receive {:trace, ^printer, :receive, result}
      assert result == %{
               onward_route: [{0, <<0, 7, 112, 114, 105, 110, 116, 101, 114>>}],
               payload: "\0\x05hello",
               return_route: [
                 {2, "\x02\a\0\d\0\0\x01\xB8\v"},
               ]
             }
    end

    test "Simple TCP Test", %{printer_pid: printer} do
      message = %{
        onward_route: [
          %TCPAddress{ip: {127, 0, 0, 1}, port: 4000},
          "printer"
        ],
        payload: "hello"
      }

      :erlang.trace(printer, true, [:receive])


      assert {:ok, _address_a} =
               Ockam.Transport.TCP.create_listener(port: 3000, route_outgoing: true)
      assert {:ok, _address_b} =
               Ockam.Transport.TCP.create_listener(port: 4000)

      Ockam.Router.route(message)

      assert_receive {:trace, ^printer, :receive, result}
      assert result == %{
               onward_route: [{0, <<0, 7, 112, 114, 105, 110, 116, 101, 114>>}],
               payload: "\0\x05hello",
               return_route: []
             }
    end
  end
end
