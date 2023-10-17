["setup.exs", "puncher.exs"] |> Enum.map(&Code.require_file/1)

[my_name, their_name, addr] = System.argv()
[rendezvous_host, port_s] = String.split(addr, ":")
{rendezvous_port, ""} = Integer.parse(port_s)

rendezvous_address = Ockam.Transport.UDPAddress.new(rendezvous_host, rendezvous_port)

{:ok, _name} =
  Puncher.create(
    address: my_name,
    attributes: %{
      target: their_name,
      rendezvous_address: rendezvous_address
    }
  )

# Port 0 will cause the OS to assign a random port
{:ok, _udp} = Ockam.Transport.UDP.start(port: 0)

Ockam.Node.whereis(my_name)
|> GenServer.call(:ping_rendezvous_server)

Process.sleep(30_000)
