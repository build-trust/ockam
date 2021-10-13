["install.exs", "forwarding_service_api.exs"] |> Enum.map(&Code.require_file/1)

Ockam.Transport.TCP.start()

hub_address = Ockam.Transport.TCPAddress.new("1.node.ockam.network", 4000)

# Register this process as address "app".
Ockam.Node.register_address("app", self())

# Register this process with a remote forwarder
{:ok, forwarder_address} = ForwardingServiceApi.register_self([hub_address, "forwarding_service"], "app")

IO.puts("Forwarder address: #{inspect(forwarder_address)}")

# Wait to receive a forwarded message
receive do
  message -> IO.puts("Address: app\t Received: #{inspect(message)}")
end
