import Config

config :logger,
  backends: [:console],
  level: :debug,
  handle_otp_reports: true,
  handle_sasl_reports: true

config :logger, :console,
  device: :user,
  level: :debug

config :ockam, :vault, curve: :curve25519

config :ockam, :transports,
  example_tcp: [
    transport: Ockam.Transport.TCP,
    listen_address: "0.0.0.0",
    listen_port: 4000
  ]

config :ockam, :services,
  influx_example: [
    service: Ockam.Services.Influx,
    database: "test",
    http: [
      host: "127.0.0.1",
      port: 8086
    ]
  ]

config :fluxter, Ockam.Services.Influx.Fluxter,
  host: "127.0.0.1",
  port: 8089,
  pool_size: 5,
  prefix: nil
