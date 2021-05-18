import Config

config :ockam_kafka,
  endpoints: [{"localhost", 9092}]

config :telemetry_influxdb,
  host: System.get_env("INFLUXDB_HOST"),
  port: System.get_env("INFLUXDB_PORT"),
  bucket: System.get_env("INFLUXDB_BUCKET"),
  org: System.get_env("INFLUXDB_ORG"),
  token: System.get_env("INFLUXDB_TOKEN")
