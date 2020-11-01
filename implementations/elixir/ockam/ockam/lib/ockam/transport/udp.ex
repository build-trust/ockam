defmodule Ockam.Transport.UDP do
  @moduledoc false

  alias Ockam.Transport.UDP.Listerner

  @doc false
  def create_listener(options) do
    Listerner.create(options)
  end
end
