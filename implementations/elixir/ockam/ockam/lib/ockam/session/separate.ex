defmodule Ockam.Session.Separate do
  @moduledoc """
  Session implementation with separate data worker.

  WIP, not all session handshake is implemented
  Please use `Ockam.Session.Pluggable`
  """
  def initiator() do
    Ockam.Session.Separate.Initiator
  end

  def responder() do
    Ockam.Session.Separate.Responder
  end
end
