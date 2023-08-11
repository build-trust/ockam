defmodule Ockam.Services.Kafka.Provider do
  # credo:disable-for-this-file Credo.Check.Refactor.ModuleDependencies

  @moduledoc """
  Implementation for Ockam.Services.Provider
  providing kafka stream services, :stream_kafka and :stream_kafka_index

  Services arguments:
  stream_kafka:
    address_prefix: optional<string>, worker address prefix
    stream_prefix: optional<string>, kafka topic prefix
    endpoints: optional<string | [string] | [{string, integer}]>, kafka bootstrap endpoints, defaults to "localhost:9092"
    user: optional<string>, kafka SASL username
    password: optional<string>, kafka SASL password, defaults to "" if only user is set
    sasl: optional<atom|string>, kafka sasl mode, defaults to "plain"
    ssl: optional<boolean>, if kafka server using ssl, defaults to false
    replication_factor: optional<integer> replication factor for topics, defaults to 1

  stream_kafka_index:
    address_prefix: optional<string>, worker address prefix
    stream_prefix: optional<string>, kafka topic prefix
    endpoints: optional<string | [string] | [{string, integer}]>, kafka bootstrap endpoints, defaults to "localhost:9092"
    user: optional<string>, kafka SASL username
    password: optional<string>, kafka SASL password, defaults to "" if only user is set
    sasl: optional<atom|string>, kafka sasl mode, defaults to "plain"
    ssl: optional<boolean> if kafka server using ssl, defaults to false

  kafka_interceptor:
    interceptor_address: optional<string>, default "kafka_interceptor" - address of interceptor spawner
    outlet_prefix: optional<string>, default "kafka_outlet_" - prefix for dynamic interceptor listeners
    authorization: optional<[Authorization]>, default [] - authorization rules for interceptors
    inlet: optional<Keyword> - optional inlet options, inlet services are not started if this config is missing
      bootstrap_port: optional<integer>, default 9000 - port on which to listen the bootstrap inlet
      base_port: optional<integer>, default 9001 - port from which to start dynamic inlets
      allowed_ports: optional<integer>, default 20 - number of ports allowed for dynamic inlets (adds to base_port)
    outlet: optional<Keyword> - optional outlet options, outlet services are not started if this config is missing
      address: optional<string>, default "kafka_bootstrap" - outlet listener address
      bootstrap_host: optional<string>, default "localhost" - hostname for kafka bootstrap server
      bootstrap_port: optional<integer>, default 9092 - port number for kafka bootstrap server
      bootstrap: optional<string>, default "localhost:9092" - host:port string for kafka bootstrap server (overrides bootstrap_host and bootstrap_port)
      ssl: optional<boolean>, default false - if ssl should be used to connect
      ssl_options: optional<list>, default [] - ssl options to use (see erlang ssl docs)
      authorization: optional<[Authorization]>, default [] - authorization rules for bootstrap outlet and spawned outlets
  """

  @behaviour Ockam.Services.Provider

  alias Ockam.Kafka.Config, as: KafkaConfig
  alias Ockam.Kafka.Interceptor.MetadataHandler

  alias Ockam.Stream.Index.Service, as: StreamIndexService
  alias Ockam.Stream.Workers.Service, as: StreamService

  @services [:stream_kafka, :stream_kafka_index, :kafka_interceptor]

  @impl true
  def services() do
    @services
  end

  @impl true
  def child_spec(:kafka_interceptor, args) do
    interceptor_childspecs(args)
  end

  def child_spec(service_name, args) do
    options = service_options(service_name, args)
    mod = service_mod(service_name)
    {mod, options}
  end

  def service_mod(:stream_kafka) do
    StreamService
  end

  def service_mod(:stream_kafka_index) do
    StreamIndexService
  end

  def service_options(:stream_kafka, args) do
    address = make_address(args, "stream_kafka")

    stream_options = [
      storage_mod: Ockam.Stream.Storage.Kafka,
      storage_options: storage_options(args)
    ]

    [address: address, stream_options: stream_options]
  end

  def service_options(:stream_kafka_index, args) do
    address = make_address(args, "stream_kafka_index")

    [
      address: address,
      storage_mod: Ockam.Stream.Index.KafkaOffset,
      storage_options: storage_options(args)
    ]
  end

  def make_address(args, default_address) do
    address_prefix = Keyword.get(args, :address_prefix, "")
    base_address = Keyword.get(args, :address, default_address)
    prefix_address(base_address, address_prefix)
  end

  def prefix_address(base_address, "") do
    base_address
  end

  def prefix_address(base_address, prefix) do
    prefix <> "_" <> base_address
  end

  def storage_options(args) do
    stream_prefix = KafkaConfig.stream_prefix(args)
    client_config = KafkaConfig.client_config(args)
    replication_factor = KafkaConfig.replication_factor(args)
    endpoints = KafkaConfig.endpoints(args)

    [
      replication_factor: replication_factor,
      endpoints: endpoints,
      client_config: client_config,
      topic_prefix: stream_prefix
    ]
  end

  def interceptor_childspecs(args) do
    outlet_args = Keyword.get(args, :outlet)
    inlet_args = Keyword.get(args, :inlet)
    interceptor_address = Keyword.get(args, :interceptor_address, "kafka_interceptor")
    outlet_prefix = Keyword.get(args, :outlet_prefix, "kafka_outlet_")
    authorization = Keyword.get(args, :authorization, [])

    outlet_childspecs = outlet_childspecs(outlet_args, outlet_prefix)

    inlet_childspecs =
      inlet_childspecs(inlet_args, outlet_args, interceptor_address, outlet_prefix)

    interceptor_childspecs = [
      {Ockam.Session.Spawner,
       [
         address: interceptor_address,
         authorization: authorization,
         worker_mod: Ockam.Transport.Portal.Interceptor,
         worker_options: [
           restart_type: :temporary,
           authorization: authorization,
           interceptor_mod: Ockam.Kafka.Interceptor,
           interceptor_options: [
             ## Options for inlet management
             handler_options: [
               base_port: Keyword.get(inlet_args || [], :base_port, 9001)
             ],
             response_handlers:
               case outlet_args do
                 nil -> []
                 _args -> [&MetadataHandler.outlet_response/3]
               end ++
                 case inlet_args do
                   nil -> []
                   _args -> [&MetadataHandler.inlet_response/3]
                 end
           ]
         ]
       ]}
    ]

    inlet_childspecs ++ outlet_childspecs ++ interceptor_childspecs
  end

  defp outlet_childspecs(nil, _outlet_prefix) do
    []
  end

  defp outlet_childspecs(outlet_args, outlet_prefix) do
    {:ok, {bootstrap_host, bootstrap_port}} =
      case Keyword.fetch(outlet_args, :bootstrap) do
        {:ok, bootstrap_str} ->
          Ockam.Transport.TCPAddress.parse_host_port(bootstrap_str)

        :error ->
          host = Keyword.get(outlet_args, :bootstrap_host, "localhost")
          port = Keyword.get(outlet_args, :bootstrap_port, 9092)
          {:ok, {host, port}}
      end

    authorization = Keyword.get(outlet_args, :authorization, [])

    address = Keyword.get(outlet_args, :address, "kafka_bootstrap")

    ssl = Keyword.get(outlet_args, :ssl, false)
    ssl_options = Keyword.get(outlet_args, :ssl_options, [])
    tcp_wrapper = Keyword.get(outlet_args, :tcp_wrapper, Ockam.Transport.TCP.DefaultWrapper)

    [
      {Ockam.Session.Spawner,
       [
         address: address,
         worker_mod: Ockam.Transport.Portal.OutletWorker,
         worker_options: [
           target_host: bootstrap_host,
           target_port: bootstrap_port,
           ssl: ssl,
           ssl_options: ssl_options,
           authorization: authorization,
           ## It doesn't make sense to restart the outlet worker
           restart_type: :temporary,
           tcp_wrapper: tcp_wrapper
         ],
         authorization: authorization
       ]},
      {Ockam.Kafka.Interceptor.OutletManager,
       [
         outlet_prefix: outlet_prefix,
         ssl: ssl,
         ssl_options: ssl_options,
         tcp_wrapper: tcp_wrapper
       ]}
    ]
  end

  defp inlet_childspecs(nil, _outlet_args, _interceptor_address, _outlet_prefix) do
    []
  end

  defp inlet_childspecs(
         inlet_args,
         outlet_args,
         interceptor_address,
         outlet_prefix
       ) do
    bootstrap_port = Keyword.get(inlet_args, :bootstrap_port, 9000)
    base_port = Keyword.get(inlet_args, :base_port, 9001)
    allowed_ports = Keyword.get(inlet_args, :allowed_ports, 20)
    tcp_wrapper = Keyword.get(inlet_args, :tcp_wrapper, Ockam.Transport.TCP.DefaultWrapper)

    outlet_route =
      Keyword.get(inlet_args, :outlet_route, [
        Keyword.get(outlet_args || [], :address, "kafka_bootstrap")
      ])

    bootstrap_route = [interceptor_address | outlet_route]
    base_route = Enum.take(bootstrap_route, length(bootstrap_route) - 1)

    [
      {
        Ockam.Transport.Portal.InletListener,
        [port: bootstrap_port, peer_route: bootstrap_route, tcp_wrapper: tcp_wrapper]
      },
      {
        Ockam.Kafka.Interceptor.InletManager,
        [
          base_port: base_port,
          allowed_ports: allowed_ports,
          base_route: base_route,
          outlet_prefix: outlet_prefix,
          tcp_wrapper: tcp_wrapper
        ]
      }
    ]
  end
end
