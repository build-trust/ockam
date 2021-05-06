defmodule Ockam.Hub do
  # credo:disable-for-this-file Credo.Check.Refactor.ModuleDependencies

  @moduledoc """
  Implements the Ockam Hub.
  """

  use Application

  alias Ockam.Hub.Service.Alias, as: AliasService
  alias Ockam.Hub.Service.Echo, as: EchoService

  alias Ockam.Stream.Client.BiDirectional.Subscribe, as: StreamSubscriptionService
  alias Ockam.Stream.Index.Worker, as: StreamIndexService
  alias Ockam.Stream.Workers.Service, as: StreamService

  alias Ockam.Hub.TelemetryForwarder
  alias Ockam.Transport

  require Logger

  # Called when the Ockam application is started.
  #
  # This function is called when an application is started using
  # `Application.start/2`, `Application.ensure_started/2` etc.
  #
  @doc false
  def start(_type, _args) do
    Logger.info("Starting Ockam Hub.")

    # Add a TCP listener on port 4000
    # TODO: add to supervision tree.
    Transport.TCP.create_listener(port: 4000)

    # Create an echo_service worker.
    # TODO: add to supervision tree.
    EchoService.create(address: "echo_service")

    AliasService.create(address: "forwarding_service")

    StreamService.create(address: "stream_service")
    StreamIndexService.create(address: "stream_index_service")

    StreamSubscriptionService.create(
      address: "stream_subscribe",
      stream_options: [
        service_route: ["stream_service"],
        index_route: ["stream_index_service"],
        partitions: 1
      ]
    )

    # on app start, create the node if it does not exist
    # we probably don't care if this errors.
    TelemetryForwarder.init()

    # Specifications of child processes that will be started and supervised.
    #
    # See the "Child specification" section in the `Supervisor` module for more
    # detailed information.
    children = [
      {
        :telemetry_poller,
        [
          period: :timer.seconds(5)
        ]
      }
    ]

    children =
      if Application.get_env(:telemetry_influxdb, :host, nil) do
        [influxdb_telemetry_config() | children]
      else
        children
      end

    # Start a supervisor with the given children. The supervisor will inturn
    # start the given children.
    #
    # The :one_for_one supervision strategy is used, if a child process
    # terminates, only that process is restarted.
    #
    # See the "Strategies" section in the `Supervisor` module for more
    # detailed information.
    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end

  defp influxdb_telemetry_config() do
    %{
      id: TelemetryInfluxDB,
      start: {
        TelemetryInfluxDB,
        :start_link,
        [
          [
            version: :v2,
            protocol: :http,
            reporter_name: "Ockam Hub",
            host: Application.get_env(:telemetry_influxdb, :host) || "http://127.0.0.1",
            port: String.to_integer(Application.get_env(:telemetry_influxdb, :port) || "8086"),
            bucket: Application.get_env(:telemetry_influxdb, :bucket) || "ockam_hub",
            org: Application.get_env(:telemetry_influxdb, :org) || "ockam",
            token: Application.get_env(:telemetry_influxdb, :token) || "TOKEN NOT CONFIGURED",
            events: [
              %{
                name: [:vm, :memory],
                metadata_tag_keys: [
                  :total,
                  :processes,
                  :processes_used,
                  :system,
                  :atom,
                  :atom_used,
                  :binary,
                  :code,
                  :ets,
                  :maximum
                ]
              },
              %{
                name: [:vm, :total_run_queue_lengths],
                metadata_tag_keys: [:total, :cpu, :io]
              },
              %{
                name: [:vm, :system_counts],
                metadata_tag_keys: [:process_count, :atom_count, :port_count]
              },
              %{
                name: [:ockam, Ockam.Transport.TCP.Listener, :init, :start],
                metadata_tag_keys: [:options, :return_value]
              },
              %{
                name: [:ockam, Ockam.Hub.Service.Echo, :init, :start],
                metadata_tag_keys: [:options, :return_value]
              },
              %{
                name: [:ockam, Ockam.Hub.Service.Alias, :init, :start],
                metadata_tag_keys: [:options, :return_value]
              },
              %{
                name: [:ockam, Ockam.Hub.Service.Alias.Forwarder, :init, :start],
                metadata_tag_keys: [:options, :return_value]
              },
              %{
                name: [:ockam, Ockam.Router, :route, :start],
                metadata_tag_keys: [:message, :return_value]
              },
              %{
                name: [:ockam, Ockam.Router, :route, :start_link],
                metadata_tag_keys: [:options, :return_value]
              },
              %{
                name: [:ockam, Ockam.Transport.TCP.Listener, :handle_message, :start],
                metadata_tag_keys: [:message, :return_value]
              },
              %{
                name: [:ockam, Ockam.Transport.UDP.Listener, :handle_message, :start],
                metadata_tag_keys: [:message, :return_value]
              },
              %{
                name: [:ockam, Ockam.Node, :handle_routed_message, :start],
                metadata_tag_keys: [:message, :return_value]
              },
              %{
                name: [:ockam, Ockam.Hub.Service.Echo, :handle_message, :start],
                metadata_tag_keys: [:message, :onward_route, :return_route, :version]
              },
              %{
                name: [:ockam, Ockam.Hub.Service.Alias, :handle_message, :start],
                metadata_tag_keys: [:message, :return_value]
              },
              %{
                name: [:ockam, Ockam.Hub.Service.Alias.Forwarder, :handle_message, :start],
                metadata_tag_keys: [:message, :return_value]
              }
            ]
          ]
        ]
      }
    }
  end
end
