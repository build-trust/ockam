defmodule Ockam.Transport.TCP do
  @moduledoc false

  alias Ockam.Transport.TCP.Client.GenTcp, as: GenTcpClient
  alias Ockam.Transport.TCP.Listener.GenTcp, as: GenTcpListener

  # if ranch is loaded, use it as the default listener
  @default_listener GenTcpListener
  if Code.ensure_loaded?(:ranch) do
    alias Ockam.Transport.TCP.Listener.Ranch, as: RanchListener
    @default_listener RanchListener
  end

  @doc false
  def create_listener(options), do: @default_listener.create(options)

  @doc false
  def create_client(options), do: GenTcpClient.create(options)
end
