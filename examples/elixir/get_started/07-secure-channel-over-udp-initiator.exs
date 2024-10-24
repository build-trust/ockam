["setup.exs"] |> Enum.map(&Code.require_file/1)

# Register this process as worker address "app".
Ockam.Node.register_address("app")

Ockam.Transport.UDP.start()

# Create an identity and a purpose key
{:ok, identity} = Ockam.Identity.create()
{:ok, keypair} = Ockam.SecureChannel.Crypto.generate_dh_keypair()
{:ok, attestation} = Ockam.Identity.attest_purpose_key(identity, keypair)

# Connect to a secure channel listener and perform a handshake.
alias Ockam.Transport.UDPAddress
r = [UDPAddress.new("127.0.0.1", 4000), "secure_channel_listener"]
{:ok, c} = Ockam.SecureChannel.create_channel(route: r,
                                              identity: identity,
                                              encryption_options: [static_keypair: keypair, static_key_attestation: attestation])

# Prepare the message.
message = %{onward_route: [c, "echoer"], return_route: ["app"], payload: "Hello Ockam!"}

# Route the message.
Ockam.Router.route(message)

# Wait to receive a reply.
receive do
  message -> IO.puts("!! Address: app\t Received: #{inspect(message)}")
end
