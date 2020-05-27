import Config

config :logger,
  level: :info,
  compile_time_purge_matching: [
    [level_lower_than: :info]
  ]
