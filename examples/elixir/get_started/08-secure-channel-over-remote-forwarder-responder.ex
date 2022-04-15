["setup.exs", "echoer.exs"] |> Enum.map(&Code.require_file/1)

# Create a Echoer type worker at address "echoer".
{:ok, _echoer} = Echoer.create(address: "echoer")

# Create a vault and an identity keypair.
{:ok, vault} = Ockam.Vault.Software.init()
{:ok, identity} = Ockam.Vault.secret_generate(vault, type: :curve25519)

# Create a secure channel listener that will wait for requests to initiate an Authenticated Key Exchange.
Ockam.SecureChannel.create_listener(vault: vault, identity_keypair: identity, address: "secure_channel_listener")

# Start the TCP Transport Add-on for Ockam Routing.
Ockam.Transport.TCP.start()

alias Ockam.Transport.TCPAddress
alias Ockam.Workers.RemoteForwarder

# Create a remote forwarder for the "app" on the node at TCPAddress - ("1.node.ockam.network", 4000)
{:ok, forwarder} = RemoteForwarder.create(
  # Route to forwarding service
  service_route: [TCPAddress.new("1.node.ockam.network", 4000), "forwarding"],
  # Route to worker to forward to
  forward_to: ["secure_channel_listener"]
)

forwarder_address = RemoteForwarder.forwarder_address(forwarder)

IO.puts("Forwarder address to secure channel listener: #{inspect(forwarder_address)}")
