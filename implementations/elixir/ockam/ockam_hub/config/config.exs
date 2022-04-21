## Application configuration used in release as sys.config or mix run
## THIS CONFIGURATION IS NOT LOADED IF THE APP IS LOADED AS A DEPENDENCY

import Config

config :ockam_hub,
  tcp_transport_port: 4000,
  udp_transport_port: 7000

config :ockam_hub,
  service_providers: [
    # default services
    Ockam.Hub.Service.Provider.Routing,
    # stream services
    Ockam.Hub.Service.Provider.Stream,
    # token lease services
    Ockam.TokenLeaseManager.Hub.Service.Provider,
    # secure channel services
    Ockam.Hub.Service.Provider.SecureChannel,
    # discovery service
    Ockam.Hub.Service.Provider.Discovery
  ]

import_config "#{Mix.env()}.exs"
