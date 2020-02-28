import Config

config :ockam, vault: []
config :ockam, transports: []

import_config "#{Mix.env()}.exs"
