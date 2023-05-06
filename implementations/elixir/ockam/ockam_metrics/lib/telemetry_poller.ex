defmodule Ockam.Metrics.TelemetryPoller do
  @moduledoc """
  Telemetry poller callbacks to collect information about workers and transports
  """

  ## TODO: maybe move this to `ockam` app?

  alias Ockam.Telemetry

  require Logger

  if Code.ensure_loaded?(Ockam.Node) do
    def dispatch_worker_count() do
      if application_started?(:ockam) do
        Enum.map(workers_by_type(), fn {type, workers} ->
          type_str = format_worker_type(type)

          Telemetry.emit_event([:workers, :type],
            measurements: %{count: Enum.count(workers)},
            metadata: %{type: type_str}
          )

          Enum.each(workers, fn address ->
            Telemetry.emit_event([:workers, :address],
              measurements: %{count: 1},
              metadata: %{type: type_str, address: address}
            )
          end)
        end)
      else
        Logger.error("Cannot report Ockam workers. :ockam application is not started")
      end
    catch
      type, error ->
        {type, error}
    end

    def dispatch_secure_channels_count() do
      ## Report secure channel initiator/responder workers
      if application_started?(:ockam) do
        %{
          handshake_initiators: handshake_initiators,
          handshake_responders: handshake_responders,
          data_initiators: data_initiators,
          data_responders: data_responders
        } = secure_channels()

        Telemetry.emit_event([:workers, :secure_channels],
          measurements: %{count: Enum.count(handshake_initiators)},
          metadata: %{type: "initiator", stage: "handshake"}
        )

        Telemetry.emit_event([:workers, :secure_channels],
          measurements: %{count: Enum.count(handshake_responders)},
          metadata: %{type: "responder", stage: "handshake"}
        )

        Telemetry.emit_event([:workers, :secure_channels],
          measurements: %{count: Enum.count(data_initiators)},
          metadata: %{type: "initiator", stage: "data"}
        )

        Telemetry.emit_event([:workers, :secure_channels],
          measurements: %{count: Enum.count(data_responders)},
          metadata: %{type: "responder", stage: "data"}
        )
      else
        Logger.error("Cannot report secure channels. :ockam application is not started")
      end
    catch
      type, error ->
        {type, error}
    end

    def secure_channels() do
      all_workers = workers_by_type()

      channel_workers = Map.get(all_workers, Ockam.SecureChannel.Channel, [])

      # Well, these workers could have already be gone at any point, this is ugly way
      # to handle that by ignoring errors caused by GenServer.call into non-existing processes.
      try_f = fn call ->
        try do
          call.()
        catch
          type, error -> {:error, {type, error}}
        end
      end

      {data_workers, handshake_workers} =
        Enum.split_with(channel_workers, fn w ->
          try_f.(fn -> Ockam.SecureChannel.established?(w) end)
        end)

      data_by_role =
        Enum.group_by(
          data_workers,
          fn address ->
            try_f.(fn -> Ockam.SecureChannel.role(address) end)
          end
        )

      handshake_by_role =
        Enum.group_by(
          handshake_workers,
          fn address ->
            try_f.(fn -> Ockam.SecureChannel.role(address) end)
          end
        )

      data_initiators = Map.get(data_by_role, :initiator, [])
      data_responders = Map.get(data_by_role, :responder, [])

      handshake_initiators = Map.get(handshake_by_role, :initiator, [])
      handshake_responders = Map.get(handshake_by_role, :responder, [])

      %{
        handshake_initiators: handshake_initiators,
        handshake_responders: handshake_responders,
        data_initiators: data_initiators,
        data_responders: data_responders
      }
    end

    defp format_worker_type(nil) do
      "Other"
    end

    defp format_worker_type(module) do
      to_string(module)
    end

    @spec workers_by_type() :: [{module(), address :: String.t()}]
    def workers_by_type() do
      Ockam.Node.list_workers()
      |> Enum.group_by(fn {_address, _pid, module} -> module end, fn {address, _pid, _modules} ->
        address
      end)
      |> Map.new()
    end
  end

  defp application_started?(app) do
    case List.keyfind(Application.started_applications(), app, 0) do
      {^app, _description, _version} -> true
      nil -> false
    end
  end
end
