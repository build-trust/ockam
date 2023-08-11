defmodule Ockam.Transport.TCP.Wrapper do
  @moduledoc """
  Behaviour definition for a TCP call wrapper.
  Allows performing additional work around TCP calls, like
  emitting telemetry events.
  """
  @callback wrap_tcp_call(transport_module :: module(), function :: atom(), args :: list()) ::
              any()
end
