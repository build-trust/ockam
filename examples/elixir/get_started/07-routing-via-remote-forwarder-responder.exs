["setup.exs", "echoer.exs"] |> Enum.map(&Code.require_file/1)

# Register this process as worker address "app".
Ockam.Node.register_address("app")

# Create a Echoer type worker at address "echoer".
{:ok, _echoer} = Echoer.create(address: "echoer")

# Start the TCP Transport Add-on for Ockam Routing.
Ockam.Transport.TCP.start()

alias Ockam.Transport.TCPAddress

# Create a remote forwarder for the "app" on the node at TCPAddress - ("1.node.ockam.network", 4000)
{:ok, forwarder} = Ockam.RemoteForwarder.create(
  # Route to forwarding service
  service_route: [TCPAddress.new("1.node.ockam.network", 4000), "forwarding_service"],
  # Route to worker to forward to
  forward_to: ["echoer"]
)

forwarder_address = Ockam.RemoteForwarder.forwarder_address(forwarder)

IO.puts("Forwarder address to echoer: #{inspect(forwarder_address)}")
