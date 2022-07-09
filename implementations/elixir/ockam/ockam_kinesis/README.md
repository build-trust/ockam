# Ockam Kinesis

AWS Kinesis storage implementation for the Ockam Stream protocol

## Installation

In order to enable Ockam Kinesis storage on your Ockam Cloud Node, add `ockam_kinesis` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:ockam_kinesis, path: "../ockam_kinesis"},
  ]
end
```

Add your AWS credentials to the application configuration:
```elixir
config :ockam_kinesis,
  access_key_id: System.get_env("AWS_ACCESS_KEY_ID"),
  secret_access_key: System.get_env("AWS_SECRET_ACCESS_KEY"),
  region: System.get_env("AWS_DEFAULT_REGION")
```

And add `Ockam.Services.Kinesis.Provider` to the list of service providers and `:ockam_kinesis` to the list of services in `ockam_services` configuration:

```elixir
config :ockam_services,
  service_providers: [
    Ockam.Services.Kinesis.Provider
  ],
  services: [
    :stream_kinesis
  ]
```

You're set! Ockam Stream service with AWS Kinesis storage backend is available at "stream_kinesis" address in your Ockam Cloud Node.
