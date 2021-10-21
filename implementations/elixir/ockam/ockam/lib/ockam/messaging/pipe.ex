defmodule Ockam.Messaging.Pipe do
  @moduledoc """
  Behaviour module for pipes
  Pipes are implemented as pairs of sender and receiver workers
  """

  @doc "Get sender module"
  @callback sender() :: module()
  @doc "Get sender module"
  @callback receiver() :: module()
end
