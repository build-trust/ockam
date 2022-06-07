defmodule Ockam.SecureChannel do
  @moduledoc """
  A secure channel provides end-to-end secure and private communication that is
  safe against eavesdropping, tampering, and forgery of messages en-route.
  """

  alias Ockam.SecureChannel.Channel
  alias Ockam.SecureChannel.Listener

  @doc """
  Create a channel listener that will create new channel responders every
  time it receives a message.
  """
  defdelegate create_listener(options), to: Listener, as: :create

  @doc """
  Create a secure channel.
  """
  defdelegate create(options), to: Channel

  @doc """
  Returns a map of information about the peer responder or initiator of a
  channel.
  """
  defdelegate peer(channel), to: Channel

  @doc """
  Returns true if the channel is established.
  """
  defdelegate established?(channel), to: Channel
end
