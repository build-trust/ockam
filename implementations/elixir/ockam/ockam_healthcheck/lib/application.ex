defmodule Ockam.Healthcheck.Application do
  @moduledoc """
  Main application for Ockam Healthcheck
  """
  use Application

  alias Ockam.Healthcheck.ScheduledTarget

  require Logger

  @doc false
  def start(_type, _args) do
    children = [Ockam.Transport.TCP] ++ healthcheck_schedule()

    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end

  def parse_config(config_json) when is_binary(config_json) do
    case Jason.decode(config_json) do
      {:ok, targets} when is_list(targets) ->
        Enum.reduce(targets, {:ok, []}, fn
          map, {:ok, targets} ->
            case ScheduledTarget.parse(map) do
              {:ok, scheduled_target} ->
                {:ok, [scheduled_target | targets]}

              {:error, reason} ->
                {:error, reason}
            end

          _map, {:error, reason} ->
            {:error, reason}
        end)

      {:ok, other} ->
        {:error, {:invalid_config, other}}

      {:error, reason} ->
        {:error, {:invalid_config, reason}}
    end
  end

  def healthcheck_schedule() do
    targets = Application.get_env(:ockam_healthcheck, :targets, [])

    Enum.map(targets, fn %ScheduledTarget{target: target, crontab: crontab} ->
      %{
        id: "#{target.name}_healthcheck_schedule",
        start:
          {SchedEx, :run_every,
           [
             Ockam.Healthcheck,
             :check_target,
             [target],
             crontab
           ]}
      }
    end)
  end
end
