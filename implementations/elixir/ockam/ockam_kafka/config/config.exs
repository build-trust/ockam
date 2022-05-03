## Application configuration used in release as sys.config or mix run
## THIS CONFIGURATION IS NOT LOADED IF THE APP IS LOADED AS A DEPENDENCY

import Config

config :ockam_kafka,
  endpoints: [{"localhost", 9092}]

config :ockam_services,
  service_providers: [Ockam.Services.Kafka.Provider]
