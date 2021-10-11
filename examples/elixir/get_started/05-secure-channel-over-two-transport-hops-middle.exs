["install.exs", "hop.exs"] |> Enum.map(&Code.require_file/1)

{:ok, _h1} = Hop.create(address: "h1")

Ockam.Transport.TCP.start(listen: [port: 3000])
