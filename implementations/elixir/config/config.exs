import Config

config :ockam, vault: []
config :ockam, transports: []
config :ockam, services: []

import_config "#{Mix.env()}.exs"
