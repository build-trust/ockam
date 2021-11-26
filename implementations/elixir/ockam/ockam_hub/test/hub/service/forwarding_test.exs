defmodule Test.Hub.Service.ForwardingTest do
  use ExUnit.Case

  alias Ockam.Hub.Service.Forwarding, as: ForwardingService

  alias Ockam.Router
  alias Test.Utils

  test "forwarding test" do
    {:ok, _worker, worker_address} =
      Test.Hub.Service.ForwardingTestWorker.start_link(address: "forwarding_test_address")

    {:ok, _forwarding, _forwarding_address} =
      ForwardingService.start_link(address: "forwarding_address")

    on_exit(fn ->
      Ockam.Node.stop("forwarding_address")
    end)

    msg = %{onward_route: [worker_address], return_route: [], payload: Utils.pid_to_string()}
    Router.route(msg)

    assert_receive(:ok, 5_000)
  end
end

defmodule Test.Hub.Service.ForwardingTestWorker do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router
  alias Test.Utils

  @forwarding_address "forwarding_address"

  @impl true
  def handle_message(message, state) do
    case Message.return_route(message) do
      [] -> registration(message, state)
      _other -> process(message, state)
    end
  end

  defp registration(message, state) do
    msg = %{
      onward_route: [@forwarding_address],
      return_route: [state.address],
      payload: ""
    }

    Router.route(msg)

    new_state =
      state
      |> Map.put(:test_process, Message.payload(message))
      |> Map.put(:status, :registered)

    {:ok, new_state}
  end

  defp process(message, state) when state.status == :registered do
    msg = Message.reply(message, state.address, "hello")

    Router.route(msg)

    new_state = Map.put(state, :status, :messaging)
    {:ok, new_state}
  end

  defp process(message, state) when state.status == :messaging do
    result =
      case Message.payload(message) do
        "hello" -> :ok
        _other -> :error
      end

    state
    |> Map.get(:test_process)
    |> Utils.string_to_pid()
    |> send(result)

    {:ok, state}
  end
end
