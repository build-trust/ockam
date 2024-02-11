["setup.exs", "hop.exs"] |> Enum.map(&Code.require_file/1)

alias Ockam.Transport.UDSAddress

{:ok, _} = Ockam.Transport.UDS.Client.create(path: "/tmp/hop.sock")

# Start the TCP Transport Add-on for Ockam Routing and a TCP listener on port 3000.
Ockam.Transport.UDS.start("/tmp/hop.sock")
