defmodule Ockam.Examples.Routing.Local do
  @moduledoc """
  Ockam Routing example

  run() - run the example
  """

  alias Ockam.Examples.Echoer
  alias Ockam.Examples.Hop

  require Logger

  def run() do
    {:ok, hop} = Hop.create([])
    {:ok, echoer} = Echoer.create([])

    ## Register this process to receive messages
    my_address = "example_run"
    Ockam.Node.register_address(my_address)

    message = %{
      ## Route message through hop to echoer
      onward_route: [hop, echoer],
      ## Trace own address in return route
      return_route: [my_address],
      payload: "HI!"
    }

    Ockam.Router.route(message)

    receive do
      %{
        onward_route: [^my_address],
        return_route: _return_route,
        payload: "HI!"
      } = message ->
        Logger.info("Received message: #{inspect(message)}")
        :ok
    end
  end
end
