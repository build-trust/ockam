## Application configuration used in release as sys.config or mix run
## THIS CONFIGURATION IS NOT LOADED IF THE APP IS LOADED AS A DEPENDENCY

import Config

config :ockam_services,
  tcp_transport_port: 4000,
  udp_transport_port: 7000

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
    Ockam.Services.Provider.Discovery
  ]

import_config "#{Mix.env()}.exs"
