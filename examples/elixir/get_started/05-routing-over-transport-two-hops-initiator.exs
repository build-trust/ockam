["setup.exs"] |> Enum.map(&Code.require_file/1)

# Register this process as worker address "app".
Ockam.Node.register_address("app")

# Start the TCP Transport Add-on for Ockam Routing.
Ockam.Transport.TCP.start()

# Prepare the message.
alias Ockam.Transport.TCPAddress
message = %{
  onward_route: [TCPAddress.new("localhost", 3000), "h1", TCPAddress.new("localhost", 4000), "echoer"],
  return_route: ["app"],
  payload: "Hello Ockam!"
}

# Route the message.
Ockam.Router.route(message)

# Wait to receive a reply.
receive do
  message -> IO.puts("Address: app\t Received: #{inspect(message)}")
end
