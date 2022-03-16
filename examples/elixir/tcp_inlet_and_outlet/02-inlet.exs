["setup.exs"] |> Enum.map(&Code.require_file/1)

# Parse argument.   Usage:  elixir 02-inlet.exs inlet_listen_port
[lport_s] = System.argv()
{lport, ""} = Integer.parse(lport_s)

Ockam.Transport.TCP.start()

{:ok, _pid} =
  Ockam.Transport.Portal.InletListener.start_link(
    port: lport,
    peer_route: [Ockam.Transport.TCPAddress.new("localhost", 4000), "outlet"]
  )

Process.sleep(:infinity)
