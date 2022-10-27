defmodule Ockam.Healthcheck.Application do
  @moduledoc """
  Main application for Ockam Healthcheck
  """

  use Application

  require Logger

  @doc false
  def start(_type, _args) do
    children = [Ockam.Transport.TCP] ++ healthcheck_schedule()

    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end

  def healthcheck_schedule() do
    ## TODO: crontab resolution is per minute
    ## maybe we can use some other tool to schedule healthchecks?
    tab = Application.get_env(:ockam_healthcheck, :crontab)
    node_host = Application.get_env(:ockam_healthcheck, :node_host)
    node_port = Application.get_env(:ockam_healthcheck, :node_port)
    api_worker = Application.get_env(:ockam_healthcheck, :api_worker)
    ping_worker = Application.get_env(:ockam_healthcheck, :ping_worker)

    case tab do
      nil ->
        []

      _string ->
        [
          %{
            id: "secure_channel_healthcheck",
            start:
              {SchedEx, :run_every,
               [
                 Ockam.Healthcheck,
                 :check_node,
                 [
                   node_host,
                   node_port,
                   api_worker,
                   ping_worker
                 ],
                 tab
               ]}
          }
        ]
    end
  end

  def check_node() do
    node_host = Application.get_env(:ockam_healthcheck, :node_host)
    node_port = Application.get_env(:ockam_healthcheck, :node_port)
    api_worker = Application.get_env(:ockam_healthcheck, :api_worker)
    ping_worker = Application.get_env(:ockam_healthcheck, :ping_worker)
    Ockam.Healthcheck.check_node(node_host, node_port, api_worker, ping_worker)
  end
end
