defmodule FileSystem.Worker do
  @moduledoc """
  FileSystem Worker Process with the backend GenServer, receive events from Port Process
  and forward it to subscribers.
  """

  use GenServer

  @doc false
  def start_link(args) do
    {opts, args} = Keyword.split(args, [:name])
    GenServer.start_link(__MODULE__, args, opts)
  end

  @doc false
  def init(args) do
    {backend, rest} = Keyword.pop(args, :backend)
    with {:ok, backend} <- FileSystem.Backend.backend(backend),
         {:ok, backend_pid} <- backend.start_link([{:worker_pid, self()} | rest])
    do
      {:ok, %{backend_pid: backend_pid, subscribers: %{}}}
    else
      _ -> :ignore
    end
  end

  @doc false
  def handle_call(:subscribe, {pid, _}, state) do
    ref = Process.monitor(pid)
    state = put_in(state, [:subscribers, ref], pid)
    {:reply, :ok, state}
  end

  @doc false
  def handle_info({:backend_file_event, backend_pid, file_event}, %{backend_pid: backend_pid}=state) do
    state.subscribers |> Enum.each(fn {_ref, subscriber_pid} ->
      send(subscriber_pid, {:file_event, self(), file_event})
    end)
    {:noreply, state}
  end

  def handle_info({:DOWN, ref, _, _pid, _reason}, state) do
    subscribers = Map.drop(state.subscribers, [ref])
    {:noreply, %{state | subscribers: subscribers}}
  end

  def handle_info(_, state) do
    {:noreply, state}
  end
end
