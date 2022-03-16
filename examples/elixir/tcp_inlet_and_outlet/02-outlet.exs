["setup.exs"] |> Enum.map(&Code.require_file/1)

# Parse argument.   Usage:  elixir 02-outlet.exs target_host:target_port
[target] = System.argv()
[host, port_s] = String.split(target, ":")
{port, ""} = Integer.parse(port_s)

Ockam.Transport.TCP.start(listen: [port: 4000])

{:ok, _spawner} =
  Ockam.Session.Spawner.create(
    address: "outlet",
    worker_mod: Ockam.Transport.Portal.OutletWorker,
    worker_options: [target_host: to_charlist(host), target_port: port]
  )

Process.sleep(:infinity)
