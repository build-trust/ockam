["setup.exs"] |> Enum.map(&Code.require_file/1)

# Register this process as worker address "app".
Ockam.Node.register_address("app")

# Start the TCP Transport Add-on for Ockam Routing.
Ockam.Transport.TCP.start()

# Ask for the address of the forwarder, that was printed by responder.
forwarder_address = IO.gets("Enter forwarder address: ") |> String.trim()

# Create a vault and an static keypair.
{:ok, vault} = Ockam.Vault.Software.init()
{:ok, keypair} = Ockam.Vault.secret_generate(vault, type: :curve25519)

# Connect to a secure channel listener and perform a handshake.
alias Ockam.Transport.TCPAddress
r = [TCPAddress.new("1.node.ockam.network", 4000), forwarder_address]
{:ok, c} = Ockam.SecureChannel.create(route: r, vault: vault, static_keypair: keypair)

# Prepare the message.
message = %{onward_route: [c, "echoer"], return_route: ["app"], payload: "Hello Ockam!"}

# Route the message.
Ockam.Router.route(message)

# Wait to receive a reply.
receive do
  message -> IO.puts("Address: app\t Received: #{inspect(message)}")
end
