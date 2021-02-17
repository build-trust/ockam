import Config

config :ockam, Ockam.Wire, default: Ockam.Wire.Binary.V2

import_config "#{Mix.env()}.exs"
