defmodule Ockam.Transport.TCP.DefaultWrapper do
  @moduledoc """
  Default implementation of a TCP wrapper behaviour.
  """
  @behaviour Ockam.Transport.TCP.Wrapper

  @impl true
  def wrap_tcp_call(transport_module, function, args \\ []) when is_list(args) do
    apply(transport_module, function, args)
  end
end
