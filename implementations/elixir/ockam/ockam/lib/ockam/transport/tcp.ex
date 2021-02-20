defmodule Ockam.Transport.TCP do
  @moduledoc false

  alias Ockam.Transport.TCP.Listener

  @spec create_listener(keyword) :: :ignore | {:error, any} | {:ok, any}
  @doc false
  def create_listener(options \\ []) do
    Listener.create(options)
  end
end
