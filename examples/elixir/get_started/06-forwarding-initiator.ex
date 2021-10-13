["install.exs"] |> Enum.map(&Code.require_file/1)

Ockam.Transport.TCP.start()

hub_address = Ockam.Transport.TCPAddress.new("1.node.ockam.network", 4000)

# Register this process as address "app".
Ockam.Node.register_address("app", self())

forwarder_address = IO.gets("Enter forwarder address:") |> String.trim()

IO.puts("Forwarder address is #{inspect(forwarder_address)}")

message = %{onward_route: [hub_address, forwarder_address], return_route: ["app"], payload: "Hello forward!"}

Ockam.Router.route(message)

:timer.sleep(500)
