["setup.exs", "echoer.exs"] |> Enum.map(&Code.require_file/1)

# Register this process as worker address "app".
Ockam.Node.register_address("app")

# Create a Echoer type worker at address "echoer".
{:ok, _echoer} = Echoer.create(address: "echoer")

# Prepare the message.
message = %{
  onward_route: ["echoer"],
  return_route: ["app"],
  payload: "Hello Ockam!"
}

# Route the message.
Ockam.Router.route(message)

# Wait to receive a reply.
receive do
  message -> IO.puts("Address: app\t Received: #{inspect(message)}")
end
