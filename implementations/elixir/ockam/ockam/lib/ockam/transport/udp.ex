defmodule Ockam.Transport.UDP do
  @moduledoc """
  UDP transport
  """

  alias Ockam.Transport.UDP.Listener

  @doc """
  Start a UDP transport

  ## Parameters
  - options:
      port: optional(integer) - port to listen on
      ip: optional(integer) - ip address to listen on
  """
  @spec start(options :: keyword) :: :ignore | {:error, any} | {:ok, any}
  def start(options \\ []) do
    Listener.start_link(options)
  end
end
