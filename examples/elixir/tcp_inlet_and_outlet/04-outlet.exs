["setup.exs"] |> Enum.map(&Code.require_file/1)

alias Ockam.Transport.TCPAddress
alias Ockam.Workers.RemoteForwarder

# Parse argument.   Usage:  elixir 03-outlet.exs target_host:target_port
[target] = System.argv()
[host, port_s] = String.split(target, ":")
{port, ""} = Integer.parse(port_s)

# Start the TCP Transport Add-on for Ockam Routing and a TCP listener on port 4000.
Ockam.Transport.TCP.start(listen: [port: 4000])

# Create a vault and an identity keypair.
{:ok, vault} = Ockam.Vault.Software.init()
{:ok, identity} = Ockam.Vault.secret_generate(vault, type: :curve25519)

# Create a secure channel listener that will wait for requests to initiate an Authenticated Key Exchange.
Ockam.SecureChannel.create_listener(
  vault: vault,
  identity_keypair: identity,
  address: "secure_channel_listener"
)

# Create a remote forwarder for the secure_channel_listener on the node at TCPAddress - ("1.node.ockam.network", 4000)
{:ok, forwarder} =
  RemoteForwarder.create(
    # Route to forwarding service
    service_route: [TCPAddress.new("1.node.ockam.network", 4000), "forwarding"],
    # Route to worker to forward to
    forward_to: ["secure_channel_listener"]
  )

forwarder_address = RemoteForwarder.forwarder_address(forwarder)
IO.puts("Forwarder address to secure channel listener: #{inspect(forwarder_address)}")

{:ok, _spawner} =
  Ockam.Session.Spawner.create(
    address: "outlet",
    worker_mod: Ockam.Transport.Portal.OutletWorker,
    worker_options: [target_host: to_charlist(host), target_port: port]
  )

Process.sleep(:infinity)
