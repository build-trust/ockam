defmodule Ockam.Services.Relay.Worker do
  @moduledoc """
  Forwards all messages to the subscribed route
  """
  use Ockam.Worker

  alias Ockam.Message

  require Logger

  def update_route(worker, route, target_identifier, tags, notify) do
    Ockam.Worker.call(worker, {:update_route, route, target_identifier, tags, notify})
  end

  @impl true
  def setup(options, state) do
    relay_options = Keyword.get(options, :relay_options, [])
    alias_str = Keyword.get(relay_options, :alias)
    user_defined_tags = Keyword.get(relay_options, :tags, %{})
    target_identifier = Keyword.get(relay_options, :target_identifier)
    notify = Keyword.get(relay_options, :notify, false)
    route = Keyword.get(relay_options, :route)
    {:ok, ts} = DateTime.now("Etc/UTC")

    regitry_metadata = %{
      service: :relay,
      tags: user_defined_tags,
      target_identifier: target_identifier,
      created_at: ts,
      updated_at: ts
    }

    maybe_notify_target(notify, route, alias_str, state.address)

    {:ok, regitry_metadata,
     Map.merge(state, %{alias: alias_str, route: route, target_identifier: target_identifier})}
  end

  @impl true
  def handle_call(
        {:update_route, route, target_identifier, user_defined_tags, notify},
        _from,
        %{alias: alias_str} = state
      ) do
    state = Map.put(state, :route, route)
    {:ok, ts} = DateTime.now("Etc/UTC")
    # Update metadata attributes
    :ok =
      Ockam.Node.update_address_metadata(
        state.address,
        fn some ->
          %{attributes: attrs} = some

          %{
            some
            | attributes: %{
                attrs
                | updated_at: ts,
                  tags: user_defined_tags,
                  target_identifier: target_identifier
              }
          }
        end
      )

    :ok = maybe_notify_target(notify, route, alias_str, state.address)
    {:reply, :ok, state}
  end

  defp maybe_notify_target(true, route, alias_str, address) do
    Ockam.Router.route(%{
      onward_route: route,
      return_route: [address],
      payload: :bare.encode("#{alias_str}", :string)
    })
  end

  defp maybe_notify_target(false, _route, _alias_str, _address), do: :ok

  @impl true
  def handle_message(message, %{route: [_ | _] = route} = state) do
    [_me | onward_route] = Message.onward_route(message)
    Ockam.Router.route(Message.set_onward_route(message, route ++ onward_route))
    {:ok, state}
  end

  def handle_message(msg, state) do
    Logger.warning("message #{inspect(msg)} received without target route setup, discarded")
    {:ok, state}
  end
end
