import Config

config :ockam_kinesis,
  access_key_id: System.get_env("AWS_ACCESS_KEY_ID"),
  secret_access_key: System.get_env("AWS_SECRET_ACCESS_KEY"),
  region: System.get_env("AWS_DEFAULT_REGION")

config :ockam_services,
  service_providers: [Ockam.Services.Kinesis.Provider],
  services: [:stream_kinesis]

import_config "#{Mix.env()}.exs"
