["setup.exs", "echoer.exs"] |> Enum.map(&Code.require_file/1)

Logger.configure(level: :debug)
# Create a Echoer type worker at address "echoer".
{:ok, _echoer} = Echoer.create(address: "echoer")



# Create an identity and a purpose key
{:ok, identity} = Ockam.Identity.create()
{:ok, keypair} = Ockam.SecureChannel.Crypto.generate_dh_keypair()
{:ok, attestation} = Ockam.Identity.attest_purpose_key(identity, keypair)

# Create a secure channel listener that will wait for requests to initiate an Authenticated Key Exchange.
Ockam.SecureChannel.create_listener(identity: identity,
                                    address: "secure_channel_listener",
                                    encryption_options: [static_keypair: keypair, static_key_attestation: attestation])

# Start the UDP Transport Add-on for Ockam Routing and a UDP listener on port 4000.
{:ok, _} = Ockam.Transport.UDP.start(port: 4000)

Process.sleep(:infinity)
