["setup.exs", "hop.exs"] |> Enum.map(&Code.require_file/1)

alias Ockam.Transport.UDSAddress

{:ok, _h1} = Hop.create(address: UDSAddress.new("/tmp/sock1"))

# Start the TCP Transport Add-on for Ockam Routing and a TCP listener on port 3000.
Ockam.Transport.UDS.start("/tmp/sock1")
