import Config

config :logger,
  level: :warn

config :fluxter, Ockam.Services.Influx.Fluxter,
  host: "127.0.0.1",
  port: 8089,
  pool_size: 5,
  prefix: nil
