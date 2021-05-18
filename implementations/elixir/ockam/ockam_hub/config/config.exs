import Config

config :logger, level: :info

config :logger, :console, metadata: [:module, :line, :pid]

import_config "#{Mix.env()}.exs"
