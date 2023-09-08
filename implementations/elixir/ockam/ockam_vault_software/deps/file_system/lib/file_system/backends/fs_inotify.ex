require Logger

defmodule FileSystem.Backends.FSInotify do
  @moduledoc """
  This file is a fork from https://github.com/synrc/fs.
  FileSystem backend for linux, freebsd and openbsd, a GenServer receive data from Port, parse event
  and send it to the worker process.
  Need `inotify-tools` installed to use this backend.

  ## Backend Options

    * `:recursive` (bool, default: true), monitor directories and their contents recursively

  ## Executable File Path

  The default behaivour to find executable file is finding `inotifywait` from `$PATH`, there're two ways to custom it, useful when run `:file_system` with escript.

    * config with `config.exs`
      `config :file_system, :fs_inotify, executable_file: "YOUR_EXECUTABLE_FILE_PATH"`

    * config with `FILESYSTEM_FSINOTIFY_EXECUTABLE_FILE` os environment
      FILESYSTEM_FSINOTIFY_EXECUTABLE_FILE=YOUR_EXECUTABLE_FILE_PATH
  """

  use GenServer
  @behaviour FileSystem.Backend
  @sep_char <<1>>

  def bootstrap do
    exec_file = executable_path()
    if is_nil(exec_file) do
      Logger.error "`inotify-tools` is needed to run `file_system` for your system, check https://github.com/rvoicilas/inotify-tools/wiki for more information about how to install it. If it's already installed but not be found, appoint executable file with `config.exs` or `FILESYSTEM_FSINOTIFY_EXECUTABLE_FILE` env."
      {:error, :fs_inotify_bootstrap_error}
    else
      :ok
    end
  end

  def supported_systems do
    [{:unix, :linux}, {:unix, :freebsd}, {:unix, :openbsd}]
  end

  def known_events do
    [:created, :deleted, :closed, :modified, :isdir, :attribute, :undefined]
  end

  defp executable_path do
    executable_path(:system_env) || executable_path(:config) || executable_path(:system_path)
  end

  defp executable_path(:config) do
    Application.get_env(:file_system, :fs_inotify)[:executable_file]
  end

  defp executable_path(:system_env) do
    System.get_env("FILESYSTEM_FSINOTIFY_EXECUTABLE_FILE")
  end

  defp executable_path(:system_path) do
    System.find_executable("inotifywait")
  end

  def parse_options(options) do
    case Keyword.pop(options, :dirs) do
      {nil, _} ->
        Logger.error "required argument `dirs` is missing"
        {:error, :missing_dirs_argument}
      {dirs, rest} ->
        format = ["%w", "%e", "%f"] |> Enum.join(@sep_char) |> to_charlist
        args = [
          '-e', 'modify', '-e', 'close_write', '-e', 'moved_to', '-e', 'moved_from',
          '-e', 'create', '-e', 'delete', '-e', 'attrib', '--format', format, '--quiet', '-m', '-r'
          | dirs |> Enum.map(&Path.absname/1) |> Enum.map(&to_charlist/1)
        ]
        parse_options(rest, args)
    end
  end

  defp parse_options([], result), do: {:ok, result}
  defp parse_options([{:recursive, true} | t], result) do
    parse_options(t, result)
  end
  defp parse_options([{:recursive, false} | t], result) do
    parse_options(t, result -- ['-r'])
  end
  defp parse_options([{:recursive, value} | t], result) do
    Logger.error "unknown value `#{inspect value}` for recursive, ignore"
    parse_options(t, result)
  end
  defp parse_options([h | t], result) do
    Logger.error "unknown option `#{inspect h}`, ignore"
    parse_options(t, result)
  end

  def start_link(args) do
    GenServer.start_link(__MODULE__, args, [])
  end

  def init(args) do
    {worker_pid, rest} = Keyword.pop(args, :worker_pid)

    case parse_options(rest) do
      {:ok, port_args} ->
        bash_args = ['-c', '#{executable_path()} "$0" "$@" & PID=$!; read a; kill -KILL $PID']

        all_args =
          case :os.type() do
            {:unix, :freebsd} ->
              bash_args ++ ['--'] ++ port_args

            _ ->
              bash_args ++ port_args
          end

        port = Port.open(
          {:spawn_executable, '/bin/sh'},
          [:stream, :exit_status, {:line, 16384}, {:args, all_args}, {:cd, System.tmp_dir!()}]
        )

        Process.link(port)
        Process.flag(:trap_exit, true)

        {:ok, %{port: port, worker_pid: worker_pid}}

      {:error, _} ->
        :ignore
    end
  end

  def handle_info({port, {:data, {:eol, line}}}, %{port: port}=state) do
    {file_path, events} = line |> parse_line
    send(state.worker_pid, {:backend_file_event, self(), {file_path, events}})
    {:noreply, state}
  end

  def handle_info({port, {:exit_status, _}}, %{port: port}=state) do
    send(state.worker_pid, {:backend_file_event, self(), :stop})
    {:stop, :normal, state}
  end

  def handle_info({:EXIT, port, _reason}, %{port: port}=state) do
    send(state.worker_pid, {:backend_file_event, self(), :stop})
    {:stop, :normal, state}
  end

  def handle_info(_, state) do
    {:noreply, state}
  end

  def parse_line(line) do
    {path, flags} =
      case line |> to_string |> String.split(@sep_char, trim: true) do
        [dir, flags, file] -> {Path.join(dir, file), flags}
        [path, flags]      -> {path, flags}
      end
    {path, flags |> String.split(",") |> Enum.map(&convert_flag/1)}
  end

  defp convert_flag("CREATE"),      do: :created
  defp convert_flag("MOVED_TO"),    do: :moved_to
  defp convert_flag("DELETE"),      do: :deleted
  defp convert_flag("MOVED_FROM"),  do: :moved_from
  defp convert_flag("ISDIR"),       do: :isdir
  defp convert_flag("MODIFY"),      do: :modified
  defp convert_flag("CLOSE_WRITE"), do: :modified
  defp convert_flag("CLOSE"),       do: :closed
  defp convert_flag("ATTRIB"),      do: :attribute
  defp convert_flag(_),             do: :undefined
end
