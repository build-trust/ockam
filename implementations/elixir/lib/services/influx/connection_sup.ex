defmodule Ockam.Services.Influx.ConnectionSupervisor do
  use DynamicSupervisor
  require Logger

  alias Ockam.Services.Influx.Connection

  def child_spec([name, opts]) do
    %{
      id: __MODULE__,
      start: {__MODULE__, :start_link, [name, opts]},
      restart: :permanent,
      shutdown: :infinity,
      type: :supervisor
    }
  end

  def start_link(name, opts) do
    DynamicSupervisor.start_link(__MODULE__, [opts], name: name)
  end

  @impl true
  def init(opts) do
    DynamicSupervisor.init(strategy: :one_for_one, extra_arguments: opts)
  end

  @spec connect(atom() | pid(), pid()) :: {:ok, Connection.t()} | {:error, term}
  def connect(sup, pid) do
    with {:ok, conn_pid} <- DynamicSupervisor.start_child(sup, {Connection, pid}) do
      {:ok, Connection.new(conn_pid, pid)}
    end
  end
end
