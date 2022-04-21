defmodule Ockam.HubNode.Cleanup.WorkerCleanup do
  @moduledoc """
  Helper functions to clean up workers by condition
  """

  def cleanup_idle_workers(module, idle_time) do
    workers = find_idle_workers(module, idle_time)
    stop_workers(workers)
  end

  def find_idle_workers(module, idle_time) when is_integer(idle_time) do
    now = System.os_time(:millisecond)

    find_workers(module, fn {_name, state} ->
      idle_worker?(state, now, idle_time)
    end)
  end

  def find_orphan_forwarders() do
    module = Ockam.Hub.Service.Forwarding.Forwarder

    filter_fun = fn {_name, state} ->
      case Map.get(state, :forward_route) do
        [first_address | _] ->
          not address_live?(first_address)

        _other ->
          true
      end
    end

    find_workers(module, filter_fun)
  end

  def address_live?(address) do
    case Ockam.Node.whereis(address) do
      pid when is_pid(pid) ->
        Process.alive?(pid)

      _other ->
        false
    end
  end

  def cleanup_workers(module, filter_fun) do
    workers = find_workers(module, filter_fun)
    stop_workers(workers)
  end

  def find_workers(module, filter_fun) do
    workers = find_module_workers(module)
    Enum.filter(workers, filter_fun)
  end

  def find_module_workers(module) do
    Ockam.Node.Registry.list_names()
    |> Enum.map(fn name ->
      state = :sys.get_state(Ockam.Node.whereis(name))

      {name, state}
    end)
    |> Enum.filter(fn {_name, state} ->
      worker_module = Map.get(state, :module)
      worker_module == module
    end)
  end

  def idle_worker?(state, now, idle_time) when is_integer(idle_time) do
    case Map.get(state, :last_message_ts) do
      nil ->
        false

      val when is_integer(val) ->
        now - val > idle_time
    end
  end

  def stop_workers(workers) do
    Enum.each(workers, fn {name, _} ->
      Ockam.Node.stop(name)
    end)
  end
end
