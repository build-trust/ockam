["install.exs"] |> Enum.map(&Code.require_file/1)

alias Ockam.Transport.TCPAddress
Ockam.Transport.TCP.start()

# Register this process as address "app".
Ockam.Node.register_address("app", self())

# Prepare our message.
message = %{onward_route: [TCPAddress.new("localhost", 4000), "echoer"], return_route: ["app"], payload: "Hello Ockam!"}

# Send the message to the worker at address "echoer".
Ockam.Router.route(message)

# Wait to receive a reply
receive do
  message -> IO.puts("Address: app\t Received: #{inspect(message)}")
end
