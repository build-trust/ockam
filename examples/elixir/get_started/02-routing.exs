["install.exs", "echoer.exs", "hop.exs"] |> Enum.map(&Code.require_file/1)

# Create a Echoer type worker at address "echoer".
{:ok, _echoer} = Echoer.create(address: "echoer")

# Create a Hop type worker at address "h1".
{:ok, _h1} = Hop.create(address: "h1")

# Register this process as address "app".
Ockam.Node.register_address("app", self())

# Prepare our message.
message = %{onward_route: ["h1", "echoer"], return_route: ["app"], payload: "Hello Ockam!"}

# Send the message to the worker at address "echoer".
Ockam.Router.route(message)

# Wait to receive a reply
receive do
  message -> IO.puts("Address: app\t Received: #{inspect(message)}")
end
