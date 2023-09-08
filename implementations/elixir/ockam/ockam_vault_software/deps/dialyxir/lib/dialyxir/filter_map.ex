defmodule Dialyxir.FilterMap do
  @moduledoc """
  A counters holding warnings to be skipped.

  `:counters` points to a `Map` where the keys are warnings to be skipped and the value indicates
  how often the warning was skipped.
  """
  defstruct list_unused_filters?: false, unused_filters_as_errors?: false, counters: %{}

  @doc """
  Fill a `FilterMap` from an ignore file.
  """
  def from_file(ignore_file, list_unused_filters?, ignore_exit_status?) do
    filter_map = %__MODULE__{
      list_unused_filters?: list_unused_filters?,
      unused_filters_as_errors?: list_unused_filters? && !ignore_exit_status?
    }

    {ignore, _} =
      ignore_file
      |> File.read!()
      |> Code.eval_string()

    Enum.reduce(ignore, filter_map, fn skip, filter_map ->
      put_in(filter_map.counters[skip], 0)
    end)
  end

  @doc """
  Remove all non-allowed arguments from `args`.
  """
  def to_args(args) do
    Keyword.take(args, [:list_unused_filters, :ignore_exit_status])
  end

  @doc """
  Retrieve the filters from a `FilterMap`.
  """
  def filters(filter_map) do
    Map.keys(filter_map.counters)
  end

  @doc """
  Increase usage count of a filter in `FilterMap`.
  """
  def inc(filter_map, filter) do
    update_in(filter_map.counters[filter], &(&1 + 1))
  end

  @doc """
  List unused filters.
  """
  def unused_filters(filter_map) do
    filter_map.counters
    |> Enum.filter(&unused?/1)
    |> Enum.unzip()
    |> elem(0)
  end

  @doc """
  Determine if any filters were not used.
  """
  def unused_filters?(filter_map) do
    Enum.any?(filter_map.counters, &unused?/1)
  end

  @doc """
  Check if a `FilterMap` entry is unused.
  """
  def unused?({_filter, count}), do: count < 1
end
