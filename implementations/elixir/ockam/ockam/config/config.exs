import Config

config :ockam, Ockam.Wire, default: Ockam.Wire.Binary.V1

import_config "#{Mix.env()}.exs"
