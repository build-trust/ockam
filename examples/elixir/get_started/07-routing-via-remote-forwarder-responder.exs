["setup.exs", "echoer.exs", "remote_forwarder.exs"] |> Enum.map(&Code.require_file/1)

# Register this process as worker address "app".
Ockam.Node.register_address("app", self())

# Create a Echoer type worker at address "echoer".
{:ok, _echoer} = Echoer.create(address: "echoer")

# Start the TCP Transport Add-on for Ockam Routing.
Ockam.Transport.TCP.start()

alias Ockam.Transport.TCPAddress

# Create a remote forwarder for the "app" on the node at TCPAddress - ("1.node.ockam.network", 4000)
{:ok, forwarder_address} = RemoteForwarder.create(
  # Route to forwarding service
  [TCPAddress.new("1.node.ockam.network", 4000), "forwarding_service"],
  # Address of worker to forward to
  "app"
)
IO.puts("Forwarder address: #{inspect(forwarder_address)}")

# Wait to receive a forwarded message.
receive do
  message -> IO.puts("Address: app\t Received: #{inspect(message)}")
end
