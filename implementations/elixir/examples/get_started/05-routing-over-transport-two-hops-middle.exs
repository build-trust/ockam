["setup.exs", "hop.exs"] |> Enum.map(&Code.require_file/1)

{:ok, _h1} = Hop.create(address: "h1")

# Start the TCP Transport Add-on for Ockam Routing and a TCP listener on port 3000.
Ockam.Transport.TCP.start(listen: [port: 3000])

Process.sleep(:infinity)
