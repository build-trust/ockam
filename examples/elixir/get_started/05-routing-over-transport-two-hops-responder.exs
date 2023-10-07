["setup.exs", "echoer.exs"] |> Enum.map(&Code.require_file/1)

# Create a Echoer type worker at address "echoer".
{:ok, _echoer} = Echoer.create(address: "echoer")

# Start the TCP Transport Add-on for Ockam Routing and a TCP listener on port 4000.
Ockam.Transport.TCP.start(listen: [port: 4000])

Process.sleep(:infinity)
