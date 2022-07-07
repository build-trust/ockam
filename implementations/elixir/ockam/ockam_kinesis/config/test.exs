import Config

config :ex_aws,
  http_client: ExAwsMock,
  access_key_id: "dummy",
  secret_access_key: "dummy"

config :ex_aws, :retries, max_attempts: 1
