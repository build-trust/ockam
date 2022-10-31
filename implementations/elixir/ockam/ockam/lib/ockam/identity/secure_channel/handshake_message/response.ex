defmodule Ockam.Identity.SecureChannel.HandshakeMessage.Response do
  @moduledoc """
  Identity channel handshake response
  """
  defstruct [:contact, :proof]
end
