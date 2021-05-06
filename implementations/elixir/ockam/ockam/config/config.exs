import Config

config :ockam, Ockam.Wire, default: Ockam.Wire.Binary.V2

config :logger, :console, metadata: [:module, :line, :pid], level: :info

import_config "#{Mix.env()}.exs"
