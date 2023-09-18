["setup.exs"] |> Enum.map(&Code.require_file/1)

# Parse argument.   Usage:  elixir 03-outlet.exs target_host:target_port
[target] = System.argv()
[host, port_s] = String.split(target, ":")
{port, ""} = Integer.parse(port_s)

# Start the TCP Transport Add-on for Ockam Routing and a TCP listener on port 4000.
Ockam.Transport.TCP.start(listen: [port: 4000])

# Create an identity and a purpose key
{:ok, identity} = Ockam.Identity.create()
{:ok, keypair} = Ockam.SecureChannel.Crypto.generate_dh_keypair()
{:ok, attestation} = Ockam.Identity.attest_purpose_key(identity, keypair)

# Create a secure channel listener that will wait for requests to initiate an Authenticated Key Exchange.
{:ok, _} = Ockam.SecureChannel.create_listener(identity: identity,
                                               address: "secure_channel_listener",
                                               encryption_options: [static_keypair: keypair, static_key_attestation: attestation])

{:ok, _spawner} =
  Ockam.Session.Spawner.create(
    address: "outlet",
    worker_mod: Ockam.Transport.Portal.OutletWorker,
    worker_options: [target_host: to_charlist(host), target_port: port]
  )

Process.sleep(:infinity)
