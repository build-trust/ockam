["setup.exs", "waiter.exs"] |> Enum.map(&Code.require_file/1)

# Register this process as worker address "app".
Ockam.Node.register_address("app")

# Start the TCP Transport Add-on for Ockam Routing.
Ockam.Transport.TCP.start()

# Create a vault and an identity keypair.
{:ok, vault} = Ockam.Vault.Software.init()
{:ok, identity} = Ockam.Vault.secret_generate(vault, type: :curve25519)

# Connect to a secure channel listener and perform a handshake.
alias Ockam.Transport.TCPAddress
r = [TCPAddress.new("localhost", 3000), "h1", TCPAddress.new("localhost", 4000), "secure_channel_listener"]
{:ok, c} = Ockam.SecureChannel.create(route: r, vault: vault, identity_keypair: identity)

# Wait for the secure channel to be established.
Waiter.wait(fn -> Ockam.SecureChannel.established?(c) end)

# Prepare the message.
message = %{onward_route: [c, "echoer"], return_route: ["app"], payload: "Hello Ockam!"}

# Route the message.
Ockam.Router.route(message)

# Wait to receive a reply.
receive do
  message -> IO.puts("Address: app\t Received: #{inspect(message)}")
end
