defmodule Ockam.Transport.UDS.ConnectionSupervisor do
  use DynamicSupervisor

  alias Ockam.Transport.UDS.Connection

  def start_link(opts) do
    name = Keyword.get(opts, :path) |> String.to_atom()
    DynamicSupervisor.start_link(__MODULE__, :no_args, name: {:global, name})
  end

  def start_child(socket, path) do
    child_spec = {Connection, socket}
    name = path |> String.to_atom()

    with {:ok, conn} <- DynamicSupervisor.start_child({:global, name}, child_spec),
         :ok <- :gen_tcp.controlling_process(socket, conn) do
      {:ok, conn}
    end
  end

  @impl true
  def init(:no_args) do
    DynamicSupervisor.init(strategy: :one_for_one, max_children: 50)
  end
end
