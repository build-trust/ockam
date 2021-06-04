defmodule Test.Hub.Service.AliasTest do
  use ExUnit.Case

  alias Ockam.Hub.Service.Alias, as: AliasService
  alias Ockam.Hub.Service.Echo, as: EchoService
  alias Ockam.Router
  alias Test.Utils

  test "alias test" do
    {:ok, _worker, worker_address} =
      Test.Hub.Service.AliasTestWorker.start_link(address: "alias_test_address")

    {:ok, _alias, _alias_address} = AliasService.start_link(address: "alias_address")
    {:ok, _echo, _echo_address} = EchoService.start_link(address: "echo_address")

    msg = %{onward_route: [worker_address], return_route: [], payload: Utils.pid_to_string()}
    Router.route(msg)

    assert_receive(:ok, 5_000)
  end
end

defmodule Test.Hub.Service.AliasTestWorker do
  @moduledoc false

  use Ockam.Worker

  alias Ockam.Message
  alias Ockam.Router
  alias Test.Utils

  @alias_address "alias_address"

  @impl true
  def handle_message(message, state) do
    case Message.return_route(message) do
      [] -> registration(message, state)
      _other -> process(message, state)
    end
  end

  defp registration(message, state) do
    msg = %{
      onward_route: [@alias_address],
      return_route: [state.address]
    }

    Router.route(msg)

    new_state =
      state
      |> Map.put(:test_process, Message.payload(message))
      |> Map.put(:status, :registered)

    {:ok, new_state}
  end

  defp process(message, state) when state.status == :registered do
    msg = %{
      onward_route: Message.return_route(message),
      return_route: [state.address],
      payload: "hello"
    }

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
