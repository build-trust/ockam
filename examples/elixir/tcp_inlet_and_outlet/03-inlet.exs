["setup.exs"] |> Enum.map(&Code.require_file/1)

# Parse argument.   Usage:  elixir 03-inlet.exs inlet_listen_port
[lport_s] = System.argv()
{lport, ""} = Integer.parse(lport_s)

Ockam.Transport.TCP.start()

# Create a vault and an identity keypair.
{:ok, vault} = Ockam.Vault.Software.init()
{:ok, identity} = Ockam.Vault.secret_generate(vault, type: :curve25519)

# Connect to a secure channel listener and perform a handshake.
r = [Ockam.Transport.TCPAddress.new("localhost", 4000), "secure_channel_listener"]
{:ok, c} = Ockam.SecureChannel.create(route: r, vault: vault, identity_keypair: identity)

{:ok, _pid} =
  Ockam.Transport.Portal.InletListener.start_link(port: lport, peer_route: [c, "outlet"])

Process.sleep(:infinity)
