defmodule Ockam.Examples.Session.CountTo.DataWorker do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message

  @impl true
  def setup(options, state) do
    count = Keyword.fetch!(options, :count)
    {:ok, Map.merge(state, %{count: count})}
  end

  @impl true
  def handle_message(message, state) do
    return_route = Message.return_route(message)

    Ockam.Worker.route(
      %{
        onward_route: return_route,
        return_route: [state.address],
        payload: "#{state.count}"
      },
      state
    )

    {:ok, state}
  end
end
