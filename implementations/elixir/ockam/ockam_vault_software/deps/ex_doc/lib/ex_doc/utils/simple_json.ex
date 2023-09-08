defmodule ExDoc.Utils.SimpleJSON do
  # We want to minimize the number of dependencies
  # ExDoc has, because we don't want someone to be allowed
  # to not upgrade their app due to an ExDoc restriction,
  # so we ship with a simple JSON implementation.
  @moduledoc false

  @doc """
  Encodes the given data to JSON in iodata form.
  """
  def encode(nil), do: "null"
  def encode(true), do: "true"
  def encode(false), do: "false"

  def encode(map) when is_map(map) do
    mapped =
      intersperse_map(Map.to_list(map), ?,, fn {key, value} ->
        [key |> Atom.to_string() |> inspect(), ?:, encode(value)]
      end)

    [?{, mapped, ?}]
  end

  def encode(list) when is_list(list) do
    mapped = intersperse_map(list, ?,, &encode/1)
    [?[, mapped, ?]]
  end

  def encode(atom) when is_atom(atom) do
    atom |> Atom.to_string() |> inspect()
  end

  def encode(binary) when is_binary(binary) do
    inspect(binary, printable_limit: :infinity)
  end

  def encode(integer) when is_integer(integer) do
    Integer.to_string(integer)
  end

  defp intersperse_map(list, separator, mapper, acc \\ [])

  defp intersperse_map([], _separator, _mapper, acc),
    do: acc

  defp intersperse_map([elem], _separator, mapper, acc),
    do: [acc | mapper.(elem)]

  defp intersperse_map([elem | rest], separator, mapper, acc),
    do: intersperse_map(rest, separator, mapper, [acc, mapper.(elem), separator])
end
