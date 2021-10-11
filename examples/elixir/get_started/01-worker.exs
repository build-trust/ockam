["install.exs", "echoer.exs"] |> Enum.map(&Code.require_file/1)

# Create a Echoer type worker at address "echoer".
{:ok, _echoer} = Echoer.create(address: "echoer")

# Register this process as address "app".
Ockam.Node.register_address("app", self())

# Prepare our message.
message = %{onward_route: ["echoer"], return_route: ["app"], payload: "Hello Ockam!"}

# Send the message to the worker at address "echoer".
Ockam.Router.route(message)

# Wait to receive a reply
receive do
  message -> IO.puts("Address: app\t Received: #{inspect(message)}")
end
