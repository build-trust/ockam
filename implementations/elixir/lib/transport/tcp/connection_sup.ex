defmodule Ockam.Transport.TCP.ConnectionSupervisor do
  use DynamicSupervisor
  require Logger

  alias Ockam.Transport.TCP.Config

  def child_spec([name, opts]) do
    %{
      id: __MODULE__,
      start: {__MODULE__, :start_link, [name, opts]},
      restart: :permanent,
      shutdown: :infinity,
      type: :supervisor
    }
  end

  def start_link(name, %Config{} = opts) do
    DynamicSupervisor.start_link(__MODULE__, [opts], name: name)
  end

  @impl true
  def init(opts) do
    DynamicSupervisor.init(strategy: :one_for_one, extra_arguments: [opts])
  end

  @spec new_connection(atom() | pid(), :gen_tcp.socket()) :: {:ok, pid} | {:error, term}
  def new_connection(sup, conn) do
    spec = {Ockam.Transport.TCP.Connection, conn}
    DynamicSupervisor.start_child(sup, spec)
  end
end
