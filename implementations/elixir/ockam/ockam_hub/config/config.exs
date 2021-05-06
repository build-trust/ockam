import Config

config :logger, level: :info

config :logger, :console, metadata: [:module, :line, :pid]

config :kafka_ex,
  disable_default_worker: true

import_config "#{Mix.env()}.exs"
