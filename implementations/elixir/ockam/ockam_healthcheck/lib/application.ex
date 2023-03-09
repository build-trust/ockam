defmodule Ockam.Healthcheck.Application do
  @moduledoc """
  Main application for Ockam Healthcheck
  """
  use Application

  alias Ockam.Healthcheck.Target

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
          target, {:ok, targets} ->
            case target do
              %{
                "name" => name,
                "host" => host,
                "port" => port
              }
              when is_integer(port) ->
                api_worker = Map.get(target, "api_worker", "api")
                healthcheck_worker = Map.get(target, "healthcheck_worker", "healthcheck")

                {:ok,
                 [
                   %Target{
                     name: name,
                     host: host,
                     port: port,
                     api_worker: api_worker,
                     healthcheck_worker: healthcheck_worker
                   }
                   | targets
                 ]}

              other ->
                {:error, {:invalid_target, other}}
            end

          _target, {:error, reason} ->
            {:error, reason}
        end)

      {:ok, other} ->
        {:error, {:invalid_config, other}}

      {:error, reason} ->
        {:error, {:invalid_config, reason}}
    end
  end

  def healthcheck_schedule() do
    tab = Application.get_env(:ockam_healthcheck, :crontab)

    case tab do
      nil ->
        []

      _string ->
        [
          %{
            id: "healthcheck_schedule",
            start:
              {SchedEx, :run_every,
               [
                 Ockam.Healthcheck,
                 :check_targets,
                 [],
                 tab
               ]}
          }
        ]
    end
  end
end
