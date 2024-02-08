["setup.exs", "rendezvous_worker.exs"] |> Enum.map(&Code.require_file/1)

require Logger

port = 5000
Logger.info("Starting Rendezvous Worker on port #{port}")

{:ok, _rendezvous} = RendezvousWorker.setup()
{:ok, _udp_t} = Ockam.Transport.UDP.start(ip: {0, 0, 0, 0}, port: port)

Process.sleep(:infinity)
