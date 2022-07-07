import Config

config :ockam_services,
  service_providers: [Ockam.Services.Kinesis.Provider],
  services: [:stream_kinesis]

import_config "#{Mix.env()}.exs"
