defmodule Ockam.Session.Separate.PreWorker do
  @moduledoc """
  Worker preparation stage.
  Used when address needs to be allocated before starting the actual worker logic

  Ignores all messages in prepare stage

  Options:

  `worker_mod` - worker to run
  `worker_options` - worker options, defaults to []
  """
  use Ockam.AsymmetricWorker

  require Logger

  @dialyzer {:nowarn_function, handle_inner_message: 2, handle_outer_message: 2}

  @doc """
  Start prepared worker with `worker_options` set in `create/2` merged with `options`
  """
  def start(worker, options) do
    Ockam.Worker.call(worker, {:start, options})
  end

  @impl true
  def inner_setup(options, state) do
    worker_mod = Keyword.fetch!(options, :worker_mod)
    worker_options = Keyword.get(options, :worker_options, [])

    base_state = state

    {:ok,
     Map.merge(state, %{
       worker_mod: worker_mod,
       worker_options: worker_options,
       base_state: base_state,
       stage: :prepare
     })}
  end

  @impl true
  def handle_message(message, %{stage: :prepare} = state) do
    Logger.warn("Ignoring message in prepare stage: #{inspect(message)}")
    {:ok, state}
  end

  def handle_message(
        message,
        %{stage: :data, worker_mod: worker_mod, data_state: data_state} = state
      ) do
    case worker_mod.handle_message(message, data_state) do
      {:ok, new_data_state} ->
        {:ok, Map.put(state, :data_state, new_data_state)}

      {:error, error} ->
        {:error, {:data_error, error}}

      {:stop, reason, new_data_state} ->
        {:stop, reason, Map.put(state, :data_state, new_data_state)}
    end
  end

  def handle_call(
        {:start, options},
        _from,
        %{
          stage: :prepare,
          worker_mod: worker_mod,
          worker_options: worker_options,
          base_state: base_state
        } = state
      ) do
    options = Keyword.merge(worker_options, options)

    case worker_mod.setup(options, base_state) do
      {:ok, data_state} ->
        {:reply, :ok, Map.merge(state, %{data_state: data_state, stage: :data})}

      {:error, err} ->
        {:reply, {:error, err}, state}
    end
  end
end
