defmodule Ockam.Transport.TCP do
  @moduledoc false

  alias Ockam.Transport.TCP.Listener

  @doc false
  def create_listener(options \\ []) do
    Listener.create(options)
  end
end
