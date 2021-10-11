["install.exs", "echoer.exs"] |> Enum.map(&Code.require_file/1)

# Create a Echoer type worker at address "echoer".
{:ok, _echoer} = Echoer.create(address: "echoer")

{:ok, vault} = Ockam.Vault.Software.init()
{:ok, identity} = Ockam.Vault.secret_generate(vault, type: :curve25519)
Ockam.SecureChannel.create_listener(vault: vault, identity_keypair: identity, address: "secure_channel_listener")

Ockam.Transport.TCP.start(listen: [port: 4000])
