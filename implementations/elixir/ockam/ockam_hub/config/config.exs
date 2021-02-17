import Config

config :logger, level: :info

import_config "#{Mix.env()}.exs"
