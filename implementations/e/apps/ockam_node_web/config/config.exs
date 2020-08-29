import Config

config :logger, :console,
  format: "$time $metadata[$level] $message\n",
  metadata: [:request_id]

config :phoenix, :json_library, Jason

config :ockam_node_web, Ockam.Node.Web.Endpoint,
  debug_errors: true,
  http: [port: 4000]
