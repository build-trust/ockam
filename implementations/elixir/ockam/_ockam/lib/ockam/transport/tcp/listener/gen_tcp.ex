defmodule Ockam.Transport.TCP.Listener.GenTcp do
  @moduledoc false

  use Ockam.Worker

  @doc false
  @impl true
  def setup(_options, state) do
    {:ok, state}
  end
end
