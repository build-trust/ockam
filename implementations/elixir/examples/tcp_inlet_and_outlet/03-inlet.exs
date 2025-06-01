["setup.exs"] |> Enum.map(&Code.require_file/1)

# Parse argument.   Usage:  elixir 03-inlet.exs inlet_listen_port
[lport_s] = System.argv()
{lport, ""} = Integer.parse(lport_s)

Ockam.Transport.TCP.start()

# Create an identity and a purpose key
{:ok, identity} = Ockam.Identity.create()
{:ok, keypair} = Ockam.SecureChannel.Crypto.generate_dh_keypair()
{:ok, attestation} = Ockam.Identity.attest_purpose_key(identity, keypair)

# Connect to a secure channel listener and perform a handshake.
r = [Ockam.Transport.TCPAddress.new("localhost", 4000), "secure_channel_listener"]
{:ok, c} = Ockam.SecureChannel.create_channel(identity: identity,
                                              route: r,
                                              encryption_options: [static_keypair: keypair, static_key_attestation: attestation])

{:ok, _pid} =
  Ockam.Transport.Portal.InletListener.start_link(port: lport, peer_route: [c, "outlet"])

Process.sleep(:infinity)
