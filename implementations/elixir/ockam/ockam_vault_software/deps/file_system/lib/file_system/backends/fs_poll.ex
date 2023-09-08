require Logger

defmodule FileSystem.Backends.FSPoll do
  @moduledoc """
  FileSysetm backend for any OS, a GenServer that regularly scans file system to
  detect changes and send them to the worker process.

  ## Backend Options

    * `:interval` (integer, default: 1000), polling interval

  ## Use FSPoll Backend

  Unlike other backends, polling backend is never automatically chosen in any
  OS environment, despite being usable on all platforms.

  To use polling backend, one has to explicitly specify in the backend option.
  """

  use GenServer
  @behaviour FileSystem.Backend

  def bootstrap, do: :ok

  def supported_systems do
    [{:unix, :linux}, {:unix, :freebsd}, {:unix, :openbsd}, {:unix, :darwin}, {:win32, :nt}]
  end

  def known_events do
    [:created, :deleted, :modified]
  end

  def start_link(args) do
    GenServer.start_link(__MODULE__, args, [])
  end

  def init(args) do
    worker_pid = Keyword.fetch!(args, :worker_pid)
    dirs = Keyword.fetch!(args, :dirs)
    interval = Keyword.get(args, :interval, 1000)

    Logger.info("Polling file changes every #{interval}ms...")
    send(self(), :first_check)

    {:ok, {worker_pid, dirs, interval, %{}}}
  end

  def handle_info(:first_check, {worker_pid, dirs, interval, _empty_map}) do
    schedule_check(interval)
    {:noreply, {worker_pid, dirs, interval, files_mtimes(dirs)}}
  end

  def handle_info(:check, {worker_pid, dirs, interval, stale_mtimes}) do
    fresh_mtimes = files_mtimes(dirs)

    diff(stale_mtimes, fresh_mtimes)
    |> Tuple.to_list
    |> Enum.zip([:created, :deleted, :modified])
    |> Enum.each(&report_change(&1, worker_pid))

    schedule_check(interval)
    {:noreply, {worker_pid, dirs, interval, fresh_mtimes}}
  end

  defp schedule_check(interval) do
    Process.send_after(self(), :check, interval)
  end

  defp files_mtimes(dirs, files_mtimes_map \\ %{}) do
    Enum.reduce(dirs, files_mtimes_map, fn dir, map ->
      case File.stat!(dir) do
        %{type: :regular, mtime: mtime} ->
          Map.put(map, dir, mtime)
        %{type: :directory} ->
          dir
          |> Path.join("*")
          |> Path.wildcard
          |> files_mtimes(map)
        %{type: _other} ->
          map
      end
    end)
  end

  @doc false
  def diff(stale_mtimes, fresh_mtimes) do
    fresh_file_paths = fresh_mtimes |> Map.keys |> MapSet.new
    stale_file_paths = stale_mtimes |> Map.keys |> MapSet.new

    created_file_paths =
      MapSet.difference(fresh_file_paths, stale_file_paths) |> MapSet.to_list
    deleted_file_paths =
      MapSet.difference(stale_file_paths, fresh_file_paths) |> MapSet.to_list
    modified_file_paths =
      for file_path <- MapSet.intersection(stale_file_paths, fresh_file_paths),
        stale_mtimes[file_path] != fresh_mtimes[file_path], do: file_path

    {created_file_paths, deleted_file_paths, modified_file_paths}
  end

  defp report_change({file_paths, event}, worker_pid) do
    for file_path <- file_paths do
      send(worker_pid, {:backend_file_event, self(), {file_path, [event]}})
    end
  end
end
