defmodule Ockam.Healthcheck.ScheduledTarget do
  @moduledoc """
  Configuration element for Ockam.Healthcheck.Application

  Defines a healthcheck target and a crontab to run the check
  """
  use TypedStruct

  alias Ockam.Healthcheck.APIEndpointTarget
  alias Ockam.Healthcheck.Target

  typedstruct enforce: true do
    field(:target, Target | APIEndpointTarget)
    field(:crontab, String.t())
  end

  @spec parse(Map.t()) :: {:ok, __MODULE__.t()} | {:error, any()}
  def parse(
        %{
          "name" => name,
          "host" => host,
          "port" => port,
          "path" => path,
          "method" => method,
          "crontab" => crontab,
          "healthcheck_worker" => healthcheck_worker
        } = target
      )
      when is_integer(port) do
    api_worker = Map.get(target, "api_worker", "api")
    body = target |> Map.get("body") |> decode_body()
    method = String.to_existing_atom(method)

    {:ok,
     %__MODULE__{
       target: %APIEndpointTarget{
         name: name,
         host: host,
         port: port,
         path: path,
         method: method,
         body: body,
         api_worker: api_worker,
         healthcheck_worker: healthcheck_worker
       },
       crontab: crontab
     }}
  end

  def parse(%{"name" => name, "host" => host, "port" => port, "crontab" => crontab} = target)
      when is_integer(port) do
    api_worker = Map.get(target, "api_worker", "api")
    healthcheck_worker = Map.get(target, "healthcheck_worker", "healthcheck")

    {:ok,
     %__MODULE__{
       target: %Target{
         name: name,
         host: host,
         port: port,
         api_worker: api_worker,
         healthcheck_worker: healthcheck_worker
       },
       crontab: crontab
     }}
  end

  def parse(other) do
    {:error, {:invalid_target, other}}
  end

  defp decode_body(nil), do: nil
  defp decode_body(body), do: Base.decode64!(body)
end
