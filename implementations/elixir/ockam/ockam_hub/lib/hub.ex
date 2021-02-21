defmodule Ockam.Hub do
  @moduledoc """
  Implements the Ockam Hub.
  """

  use Application

  alias Ockam.Hub.Service.Echo, as: EchoService
  alias Ockam.Transport.TCP

  require Logger

  # Called when the Ockam application is started.
  #
  # This function is called when an application is started using
  # `Application.start/2`, `Application.ensure_started/2` etc.
  #
  @doc false
  def start(_type, _args) do
    Logger.info("Starting Ockam Hub.")

    # Add a TCP listener on port 4000
    # TODO: add to supervision tree.
    TCP.create_listener(port: 4000)

    # Create an echo_service worker.
    # TODO: add to supervision tree.
    EchoService.create(address: "echo_service")

    # Specifications of child processes that will be started and supervised.
    #
    # See the "Child specification" section in the `Supervisor` module for more
    # detailed information.
    children = []

    # Start a supervisor with the given children. The supervisor will inturn
    # start the given children.
    #
    # The :one_for_one supervision strategy is used, if a child process
    # terminates, only that process is restarted.
    #
    # See the "Strategies" section in the `Supervisor` module for more
    # detailed information.
    Supervisor.start_link(children, strategy: :one_for_one, name: __MODULE__)
  end
end
