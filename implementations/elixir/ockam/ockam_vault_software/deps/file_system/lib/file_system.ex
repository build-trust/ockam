defmodule FileSystem do
  @moduledoc File.read!("README.md")

  @doc """
  ## Options

    * `:dirs` ([string], required), the dir list to monitor

    * `:backend` (atom, optional), default backends: `:fs_mac`
      for `macos`, `:fs_inotify` for `linux`, `freebsd` and `openbsd`,
      `:fs_windows` for `windows`

    * `:name` (atom, optional), `name` can be used to subscribe as
      the same as pid when the `name` is given. The `name` should
      be the name of worker process.

    * All rest options will treated as backend options. See backend
      module documents for more details.

  ## Example

  Simple usage:

      iex> {:ok, pid} = FileSystem.start_link(dirs: ["/tmp/fs"])
      iex> FileSystem.subscribe(pid)

  Get instant notifications on file changes for Mac OS X:

      iex> FileSystem.start_link(dirs: ["/path/to/some/files"], latency: 0)

  Named monitor with specified backend:

      iex> FileSystem.start_link(backend: :fs_mac, dirs: ["/tmp/fs"], name: :worker)
      iex> FileSystem.subscribe(:worker)
  """
  @spec start_link(Keyword.t) :: GenServer.on_start()
  def start_link(options) do
    FileSystem.Worker.start_link(options)
  end

  @doc """
  Register the current process as a subscriber of a file_system worker.
  The pid you subscribed from will now receive messages like

      {:file_event, worker_pid, {file_path, events}}
      {:file_event, worker_pid, :stop}
  """
  @spec subscribe(GenServer.server) :: :ok
  def subscribe(pid) do
    GenServer.call(pid, :subscribe)
  end
end
