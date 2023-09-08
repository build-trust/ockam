defmodule Jason.Sigil do
  @doc ~S"""
  Handles the sigil `~j` for JSON strings.

  Calls `Jason.decode!/2` with modifiers mapped to options.

  Given a string literal without interpolations, decodes the
  string at compile-time.

  ## Modifiers

  See `Jason.decode/2` for detailed descriptions.

    * `a` - equivalent to `{:keys, :atoms}` option
    * `A` - equivalent to `{:keys, :atoms!}` option
    * `r` - equivalent to `{:strings, :reference}` option
    * `c` - equivalent to `{:strings, :copy}` option

  ## Examples

      iex> ~j"0"
      0

      iex> ~j"[1, 2, 3]"
      [1, 2, 3]

      iex> ~j'"string"'r
      "string"

      iex> ~j"{}"
      %{}

      iex> ~j'{"atom": "value"}'a
      %{atom: "value"}

      iex> ~j'{"#{:j}": #{'"j"'}}'A
      %{j: "j"}

  """
  defmacro sigil_j(term, modifiers)

  defmacro sigil_j({:<<>>, _meta, [string]}, modifiers) when is_binary(string) do
    Macro.escape(Jason.decode!(string, mods_to_opts(modifiers)))
  end

  defmacro sigil_j(term, modifiers) do
    quote(do: Jason.decode!(unquote(term), unquote(mods_to_opts(modifiers))))
  end

  @doc ~S"""
  Handles the sigil `~J` for raw JSON strings.

  Decodes a raw string ignoring Elixir interpolations and
  escape characters at compile-time.

  ## Examples

      iex> ~J'"#{string}"'
      "\#{string}"

      iex> ~J'"\u0078\\y"'
      "x\\y"

      iex> ~J'{"#{key}": "#{}"}'a
      %{"\#{key}": "\#{}"}
  """
  defmacro sigil_J(term, modifiers)

  defmacro sigil_J({:<<>>, _meta, [string]}, modifiers) when is_binary(string) do
    Macro.escape(Jason.decode!(string, mods_to_opts(modifiers)))
  end

  @spec mods_to_opts(charlist) :: [Jason.decode_opt()]
  defp mods_to_opts(modifiers) do
    modifiers
    |> Enum.map(fn
      ?a -> {:keys, :atoms}
      ?A -> {:keys, :atoms!}
      ?r -> {:strings, :reference}
      ?c -> {:strings, :copy}
      m -> raise ArgumentError, "unknown sigil modifier #{<<?", m, ?">>}"
    end)
  end
end
