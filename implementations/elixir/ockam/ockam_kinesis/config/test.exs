import Config

config :ockam_kinesis,
  http_client: {AWSMock, []},
  access_key_id: "dummy",
  secret_access_key: "dummy",
  region: "us-east-1"
