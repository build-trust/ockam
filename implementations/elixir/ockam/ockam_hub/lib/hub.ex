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
            period: :timer.seconds(30),
            measurements: [
              {Ockam.Hub.TelemetryPoller, :dispatch_worker_count, []},
              {Ockam.Hub.TelemetryPoller, :dispatch_tcp_connections, []}
            ]
          ]
        },
        # Add a TCP listener
        {Ockam.Transport.TCP, [listen: [port: tcp_transport_port]]},
        # Add a UDP listener
        {Ockam.Transport.UDP.Listener,
         [
           port: udp_transport_port
         ]},
        Ockam.Hub.Service.Provider
      ] ++
        schedule_specs ++
        if Application.get_env(:telemetry_influxdb, :host, nil) do
          [Ockam.Hub.Metrics.TelemetryInfluxDB.child_spec()]
        else
          []
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
end
