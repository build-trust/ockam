require Logger

defmodule FileSystem.Backends.FSWindows do
  @moduledoc """
  This file is a fork from https://github.com/synrc/fs.
  FileSysetm backend for windows, a GenServer receive data from Port, parse event
  and send it to the worker process.
  Need binary executable file packaged in to use this backend.

  ## Backend Options

    * `:recursive` (bool, default: true), monitor directories and their contents recursively

  ## Executable File Path

  The default executable file is `inotifywait.exe` in `priv` dir of `:file_system` application, there're two ways to custom it, useful when run `:file_system` with escript.

    * config with `config.exs`
      `config :file_system, :fs_windows, executable_file: "YOUR_EXECUTABLE_FILE_PATH"`

    * config with `FILESYSTEM_FSWINDOWS_EXECUTABLE_FILE` os environment
      FILESYSTEM_FSWINDOWS_EXECUTABLE_FILE=YOUR_EXECUTABLE_FILE_PATH
  """

  use GenServer
  @behaviour FileSystem.Backend
  @sep_char <<1>>

  @default_exec_file "inotifywait.exe"

  def bootstrap do
    exec_file = executable_path()
    if not is_nil(exec_file) and File.exists?(exec_file) do
      :ok
    else
      Logger.error "Can't find executable `inotifywait.exe`"
      {:error, :fs_windows_bootstrap_error}
    end
  end

  def supported_systems do
    [{:win32, :nt}]
  end

  def known_events do
    [:created, :modified, :removed, :renamed, :undefined]
  end

  defp executable_path do
    executable_path(:system_env) || executable_path(:config) || executable_path(:system_path) || executable_path(:priv)
  end

  defp executable_path(:config) do
    Application.get_env(:file_system, :fs_windows)[:executable_file]
  end

  defp executable_path(:system_env) do
    System.get_env("FILESYSTEM_FSMWINDOWS_EXECUTABLE_FILE")
  end

  defp executable_path(:system_path) do
    System.find_executable(@default_exec_file)
  end

  defp executable_path(:priv) do
    case :code.priv_dir(:file_system) do
      {:error, _} ->
        Logger.error "`priv` dir for `:file_system` application is not avalible in current runtime, appoint executable file with `config.exs` or `FILESYSTEM_FSWINDOWS_EXECUTABLE_FILE` env."
        nil
      dir when is_list(dir) ->
        Path.join(dir, @default_exec_file)
    end
  end

  def parse_options(options) do
    case Keyword.pop(options, :dirs) do
      {nil, _} ->
        Logger.error "required argument `dirs` is missing"
        {:error, :missing_dirs_argument}
      {dirs, rest} ->
        format = ["%w", "%e", "%f"] |> Enum.join(@sep_char) |> to_charlist
        args = [
          '--format', format, '--quiet', '-m', '-r'
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
        port = Port.open(
          {:spawn_executable, to_charlist(executable_path())},
          [:stream, :exit_status, {:line, 16384}, {:args, port_args}, {:cd, System.tmp_dir!()}]
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
        [dir, flags, file] -> {Enum.join([dir, file], "\\"), flags}
        [path, flags]      -> {path, flags}
      end
    {path |> Path.split() |> Path.join(), flags |> String.split(",") |> Enum.map(&convert_flag/1)}
  end

  defp convert_flag("CREATE"),   do: :created
  defp convert_flag("MODIFY"),   do: :modified
  defp convert_flag("DELETE"),   do: :removed
  defp convert_flag("MOVED_TO"), do: :renamed
  defp convert_flag(_),          do: :undefined
end
