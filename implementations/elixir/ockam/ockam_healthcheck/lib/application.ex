defmodule Ockam.Healthcheck.Application do
  @moduledoc """
  Main application for Ockam Healthcheck
  """
  use Application

  alias Ockam.Healthcheck.APIEndpointTarget
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
          %{
            "name" => name,
            "host" => host,
            "port" => port,
            "path" => path,
            "method" => method,
            "crontab" => crontab,
            "healthcheck_worker" => healthcheck_worker
          } = target,
          {:ok, targets}
          when is_integer(port) ->
            api_worker = Map.get(target, "api_worker", "api")
            body = target |> Map.get("body") |> decode_body()
            method = String.to_existing_atom(method)

            {:ok,
             [
               %APIEndpointTarget{
                 name: name,
                 host: host,
                 port: port,
                 path: path,
                 method: method,
                 body: body,
                 api_worker: api_worker,
                 healthcheck_worker: healthcheck_worker,
                 crontab: crontab
               }
               | targets
             ]}

          %{"name" => name, "host" => host, "port" => port, "crontab" => crontab} = target,
          {:ok, targets}
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
                 healthcheck_worker: healthcheck_worker,
                 crontab: crontab
               }
               | targets
             ]}

          other, {:ok, _targets} ->
            {:error, {:invalid_target, other}}

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
    targets = Application.get_env(:ockam_healthcheck, :targets, [])

    Enum.reduce(targets, [], fn target, acc ->
      [
        %{
          id: "#{target.name}_healthcheck_schedule",
          start:
            {SchedEx, :run_every,
             [
               Ockam.Healthcheck,
               :check_target,
               [target],
               target.crontab
             ]}
        }
        | acc
      ]
    end)
  end

  defp decode_body(nil), do: nil
  defp decode_body(body), do: Base.decode64!(body)
end
