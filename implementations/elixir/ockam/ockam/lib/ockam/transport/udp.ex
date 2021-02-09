defmodule Ockam.Transport.UDP do
  @moduledoc false

  alias Ockam.Transport.UDP.Listener

  @doc false
  def create_listener(options \\ []) do
    Listener.create(options)
  end
end
