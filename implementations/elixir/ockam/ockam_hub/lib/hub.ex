defmodule Ockam.Hub do
  # credo:disable-for-this-file Credo.Check.Refactor.ModuleDependencies

  @moduledoc """
  Implements the Ockam Hub.
  """

  use Application

  require Logger

  # Called when the Ockam application is started.
  #
  # This function is called when an application is started using
  # `Application.start/2`, `Application.ensure_started/2` etc.
  #
  @doc false
  def start(_type, _args) do
    Logger.info("Starting Ockam Hub.")

    tcp_transport_port = Application.get_env(:ockam_hub, :tcp_transport_port)
    udp_transport_port = Application.get_env(:ockam_hub, :udp_transport_port)

    ## Get configured services child specs
    {:ok, services_specs} = Ockam.Hub.Service.Provider.configured_child_specs()

    schedule_specs = cleanup_schedule(Application.get_env(:ockam_hub, :cleanup))

    # Specifications of child processes that will be started and supervised.
    #
    # See the "Child specification" section in the `Supervisor` module for more
    # detailed information.
    children =
      [
        {
          :telemetry_poller,
          [
            period: :timer.seconds(5)
          ]
        },
        # Add a TCP listener
        {Ockam.Transport.TCP, [listen: [port: tcp_transport_port]]},
        # Add a UDP listener
        {Ockam.Transport.UDP.Listener,
         [
           port: udp_transport_port
         ]}
      ] ++
        services_specs ++
        schedule_specs

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

  defp cleanup_schedule(config) do
    crontab = Keyword.get(config, :crontab)
    idle_timeout = Keyword.get(config, :idle_timeout)
    cleanup_kafka_topics = Keyword.get(config, :cleanup_kafka_topics)

    case {crontab, idle_timeout} do
      {tab, timeout} when is_binary(tab) and is_integer(timeout) ->
        case Crontab.CronExpression.Parser.parse(tab) do
          {:ok, _} ->
            [
              %{
                id: "cleanup",
                start:
                  {SchedEx, :run_every,
                   [Ockam.Hub.Cleanup, :cleanup_all, [timeout, cleanup_kafka_topics], tab]}
              }
            ]

          other ->
            Logger.info("Invalid cleanup crontab: #{inspect(tab)} : #{inspect(other)}. Ignoring")
            []
        end

      _other ->
        Logger.info(
          "Invalid cleanup config: #{inspect(crontab)} : #{inspect(idle_timeout)}. Ignoring"
        )

        []
    end
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
            tags: %{hostname: System.get_env("HOSTNAME", "none")},
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
                name: [:ockam, Ockam.Node, :handle_local_message, :start],
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
