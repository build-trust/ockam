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
                path = Map.get(target, "path")
                method = target |> Map.get("method", "nil") |> String.to_existing_atom()
                body = Map.get(target, "body")

                {:ok,
                 [
                   %Target{
                     name: name,
                     host: host,
                     port: port,
                     path: path,
                     method: method,
                     body: body,
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
    api_endpoint_tab = Application.get_env(:ockam_healthcheck, :api_crontab)

    default_schedule =
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

    api_endpoint_schedule =
      case api_endpoint_tab do
        nil ->
          []

        _string ->
          [
            %{
              id: "api_healthcheck_schedule",
              start:
                {SchedEx, :run_every,
                 [
                   Ockam.Healthcheck,
                   :check_api_endpoints,
                   [],
                   api_endpoint_tab
                 ]}
            }
          ]
      end

    default_schedule ++ api_endpoint_schedule
  end
end
