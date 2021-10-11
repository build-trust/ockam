["install.exs", "waiter.exs"] |> Enum.map(&Code.require_file/1)

alias Ockam.Transport.TCPAddress
Ockam.Transport.TCP.start()

{:ok, vault} = Ockam.Vault.Software.init()
{:ok, identity} = Ockam.Vault.secret_generate(vault, type: :curve25519)

r = [TCPAddress.new("localhost", 3000), "h1", TCPAddress.new("localhost", 4000), "secure_channel_listener"]
{:ok, c} = Ockam.SecureChannel.create(route: r, vault: vault, identity_keypair: identity)

Waiter.wait(fn -> Ockam.SecureChannel.established?(c) end)

# Register this process as address "app".
Ockam.Node.register_address("app", self())

# Prepare our message.
message = %{onward_route: [c, "echoer"], return_route: ["app"], payload: "Hello Ockam!"}

# Send the message to the worker at address "echoer".
Ockam.Router.route(message)

# Wait to receive a reply
receive do
  message -> IO.puts("Address: app\t Received: #{inspect(message)}")
end
