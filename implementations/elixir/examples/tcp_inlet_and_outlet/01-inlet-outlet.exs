["setup.exs"] |> Enum.map(&Code.require_file/1)

# Parse argument.   Usage:  elixir 01-inlet-outlet.exs inlet_liste_port target_host:target_port
[lport_s, target] = System.argv()
{lport, ""} = Integer.parse(lport_s)
[host, port_s] = String.split(target, ":")
{port, ""} = Integer.parse(port_s)

{:ok, _pid} = Ockam.Transport.Portal.InletListener.start_link(port: lport, peer_route: ["outlet"])

{:ok, _spawner} =
  Ockam.Session.Spawner.create(
    address: "outlet",
    worker_mod: Ockam.Transport.Portal.OutletWorker,
    worker_options: [target_host: to_charlist(host), target_port: port]
  )

Process.sleep(:infinity)
