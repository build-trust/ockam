defmodule Ockam.SecureChannel do
  @moduledoc """
  A secure channel provides end-to-end secure and private communication that is
  safe against eavesdropping, tampering, and forgery of messages en-route.
  """

  alias Ockam.SecureChannel.Channel

  @doc """
  Create a channel listener that will create new channel responders every
  time it receives a message.
  """
  defdelegate create_listener(options), to: Channel

  @doc """
  Create a secure channel.
  """
  defdelegate create_channel(options, handshake_timeout \\ 30_000), to: Channel
  defdelegate start_link_channel(options, handshake_timeout \\ 30_000), to: Channel
  defdelegate get_remote_identity(worker), to: Channel
  defdelegate get_remote_identity_id(worker), to: Channel
  defdelegate get_remote_identity_with_id(worker), to: Channel
  defdelegate listener_child_spec(args), to: Channel
  defdelegate established?(worker), to: Channel
  defdelegate disconnect(worker), to: Channel
  defdelegate role(worker), to: Channel
end
