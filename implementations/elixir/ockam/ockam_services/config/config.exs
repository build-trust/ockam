## Application configuration used in release as sys.config or mix run
## THIS CONFIGURATION IS NOT LOADED IF THE APP IS LOADED AS A DEPENDENCY

import Config

config :ockam_services,
  tcp_transport: [listen: [port: 4000]],
  udp_transport: [port: 7000]

config :ockam_services,
  service_providers: [
    # default services
    Ockam.Services.Provider.Routing,
    # stream services
    Ockam.Services.Provider.Stream,
    # token lease services
    Ockam.Services.TokenLeaseManager.Provider,
    # secure channel services
    Ockam.Services.Provider.SecureChannel,
    # discovery service
    Ockam.Services.Provider.Discovery,
    Ockam.Services.Provider.NodeInfo,
    # Rust sidecar services
    Ockam.Services.Provider.Sidecar
  ]

import_config "#{Mix.env()}.exs"
