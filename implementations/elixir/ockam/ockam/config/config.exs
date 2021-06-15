import Config

config :ockam, Ockam.Wire, default: Ockam.Wire.Binary.V2

config :logger, :console, metadata: [:module, :line, :pid], level: :info
# config :logger, handle_sasl_reports: true

import_config "#{Mix.env()}.exs"
