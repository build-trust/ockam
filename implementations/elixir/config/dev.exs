import Config

config :logger,
  backends: [:console],
  level: :info,
  handle_otp_reports: true,
  handle_sasl_reports: true

config :logger, :console,
  device: :user,
  level: :info

config :ockam, :vault, curve: :curve25519

config :ockam, :transports,
  example_tcp: [
    transport: Ockam.Transport.TCP,
    listen_address: "0.0.0.0",
    listen_port: 4000
  ]
