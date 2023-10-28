defmodule Ockam.Session.Pluggable do
  @moduledoc """
  Routing session

  Initiator sends a first handshake message to the init_route on start
  and waits for a response from the responder.

  Responder receives the handshake, starts the data worker and sends a response

  After receiving the response, initiator starts the data worker.

  Utilizing pluggable handshake logic using `Ockam.Session.Handshake` behaviour
  """

  def initiator() do
    Ockam.Session.Pluggable.Initiator
  end

  def responder() do
    Ockam.Session.Pluggable.Responder
  end

  def handshake_mod(server) do
    Ockam.Worker.call(server, :get_handshake_mod)
  end

  def data_worker_mod(server) do
    Ockam.Worker.call(server, :get_data_worker_mod)
  end

  def role(server) do
    Ockam.Worker.call(server, :get_role)
  end

  @doc """
  Shared function for data stage of the session

  State MUST have :data_state and :data_worker_mod keys
  """
  @spec handle_data_message(any(), %{
          :data_state => any(),
          :data_worker_mod => atom(),
          any() => any()
        }) ::
          {:ok, %{data_state: any()}} | {:error, any()} | {:stop, any(), %{data_state: any()}}
  def handle_data_message(message, state) do
    data_state = Map.fetch!(state, :data_state)
    data_worker_mod = Map.fetch!(state, :data_worker_mod)

    case data_worker_mod.handle_message(message, data_state) do
      {:ok, new_data_state} ->
        {:ok, update_data_state(state, new_data_state)}

      {:error, error} ->
        {:error, {:data_error, error}}

      {:stop, reason, new_data_state} ->
        {:stop, reason, update_data_state(state, new_data_state)}
    end
  end

  def handle_call(:get_handshake_mod, _from, state) do
    handshake_mod = Map.fetch!(state, :handshake_mod)
    {:reply, handshake_mod, state}
  end

  def handle_call(:get_data_worker_mod, _from, state) do
    data_worker_mod = Map.fetch!(state, :data_worker_mod)
    {:reply, data_worker_mod, state}
  end

  def handle_call(call, from, %{stage: :handshake} = state) do
    handshake_mod = Map.fetch!(state, :handshake_mod)
    handshake_state = Map.fetch!(state, :handshake_state)

    case handshake_mod.handle_call(call, from, handshake_state) do
      {:reply, reply, handshake_state} ->
        {:reply, reply, update_handshake_state(state, handshake_state)}

      {:reply, reply, handshake_state, extra} ->
        {:reply, reply, update_handshake_state(state, handshake_state), extra}

      {:noreply, handshake_state} ->
        {:noreply, update_handshake_state(state, handshake_state)}

      {:noreply, handshake_state, extra} ->
        {:noreply, update_handshake_state(state, handshake_state), extra}

      {:stop, reason, reply, handshake_state} ->
        {:stop, reason, reply, update_handshake_state(state, handshake_state)}

      {:stop, reason, handshake_state} ->
        {:stop, reason, update_handshake_state(state, handshake_state)}
    end
  end

  def handle_call(call, from, %{stage: :data} = state) do
    data_state = Map.fetch!(state, :data_state)
    data_worker_mod = Map.fetch!(state, :data_worker_mod)

    case data_worker_mod.handle_call(call, from, data_state) do
      {:reply, reply, data_state} ->
        {:reply, reply, update_data_state(state, data_state)}

      {:reply, reply, data_state, extra} ->
        {:reply, reply, update_data_state(state, data_state), extra}

      {:noreply, data_state} ->
        {:noreply, update_data_state(state, data_state)}

      {:noreply, data_state, extra} ->
        {:noreply, update_data_state(state, data_state), extra}

      {:stop, reason, reply, data_state} ->
        {:stop, reason, reply, update_data_state(state, data_state)}

      {:stop, reason, data_state} ->
        {:stop, reason, update_data_state(state, data_state)}
    end
  end

  ## Helper functions used by initiator and responder

  def switch_to_data_stage(message \\ nil, start_options, handshake_state, state) do
    base_state = Map.fetch!(state, :base_state)
    data_worker_mod = Map.fetch!(state, :data_worker_mod)
    data_worker_options = Map.fetch!(state, :data_worker_options)

    ## Set registered module to data mod
    set_module(state, data_worker_mod)

    options = Keyword.merge(data_worker_options, start_options)

    case data_worker_mod.setup(options, base_state) do
      {:ok, data_state} ->
        case message do
          nil -> :ok
          %{} -> Ockam.Worker.route(message, base_state)
        end

        state =
          state
          |> update_handshake_state(handshake_state)
          |> update_data_state(data_state)
          |> Map.put(:stage, :data)

        {:ok, state}

      {:error, err} ->
        {:stop, {:cannot_start_data_worker, {:error, err}, options, handshake_state, base_state},
         state}
    end
  end

  def update_handshake_state(state, handshake_state) do
    update_export_fields(state, handshake_state) |> Map.put(:handshake_state, handshake_state)
  end

  defp update_data_state(state, data_state) do
    update_export_fields(state, data_state) |> Map.put(:data_state, data_state)
  end

  defp update_export_fields(state, internal_state) do
    base_state = Map.fetch!(state, :base_state)

    all_addresses =
      Map.get(internal_state, :all_addresses, [])
      |> Enum.concat(Map.get(base_state, :all_addresses, []))
      |> Enum.uniq()

    internal_auth = Map.get(internal_state, :authorization, [])
    base_auth = Map.get(base_state, :authorization, [])

    authorization =
      case {internal_auth, base_auth} do
        {list, also_list} when is_list(list) and is_list(also_list) ->
          merge_list(list, also_list)

        {map, also_map} when is_map(map) and is_map(also_map) ->
          Map.merge(map, also_map, fn _key, internal_val, base_val ->
            merge_list(internal_val, base_val)
          end)

        {map, list} when is_map(map) and is_list(list) ->
          merge_list_and_map(list, map, all_addresses)

        {list, map} when is_map(map) and is_list(list) ->
          merge_list_and_map(list, map, all_addresses)
      end

    Map.merge(state, %{
      authorization: authorization,
      all_addresses: all_addresses
    })
  end

  def set_module(state, module) do
    all_addresses = Map.get(state, :all_addresses, [])

    Enum.each(all_addresses, fn address ->
      Ockam.Node.update_address_metadata(address, fn _prev ->
        %{module: module, attributes: %{}}
      end)
    end)
  end

  defp merge_list(list1, list2) do
    Enum.uniq(list1 ++ list2)
  end

  def merge_list_and_map(list, map, all_addresses) do
    Enum.map(all_addresses, fn address ->
      map_config = Map.get(map, address, [])
      {address, merge_list(map_config, list)}
    end)
    |> Map.new()
  end
end
