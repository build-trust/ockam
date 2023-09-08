defmodule Erlex do
  @moduledoc """
  Convert Erlang style structs and error messages to equivalent Elixir.

  Lexes and parses the Erlang output, then runs through pretty
  printer.

  ## Usage

  Invoke `Erlex.pretty_print/1` wuth the input string.

  ```elixir
  iex> str = ~S"('Elixir.Plug.Conn':t(),binary() | atom(),'Elixir.Keyword':t() | map()) -> 'Elixir.Plug.Conn':t()"
  iex> Erlex.pretty_print(str)
  (Plug.Conn.t(), binary() | atom(), Keyword.t() | map()) :: Plug.Conn.t()
  ```

  While the lion's share of the work is done via invoking
  `Erlex.pretty_print/1`, other higher order functions exist for further
  formatting certain messages by running through the Elixir formatter.
  Because we know the previous example is a type, we can invoke the
  `Erlex.pretty_print_contract/1` function, which would format that
  appropriately for very long lines.

  ```elixir
  iex> str = ~S"('Elixir.Plug.Conn':t(),binary() | atom(),'Elixir.Keyword':t() | map(), map() | atom(), non_neg_integer(), binary(), binary(), binary(), binary(), binary()) -> 'Elixir.Plug.Conn':t()"
  iex> Erlex.pretty_print_contract(str)
  (
    Plug.Conn.t(),
    binary() | atom(),
    Keyword.t() | map(),
    map() | atom(),
    non_neg_integer(),
    binary(),
    binary(),
    binary(),
    binary(),
    binary()
  ) :: Plug.Conn.t()
  ```
  """

  defp lex(str) do
    try do
      {:ok, tokens, _} = :lexer.string(str)
      tokens
    rescue
      _ ->
        throw({:error, :lexing, str})
    end
  end

  defp parse(tokens) do
    try do
      {:ok, [first | _]} = :parser.parse(tokens)
      first
    rescue
      _ ->
        throw({:error, :parsing, tokens})
    end
  end

  defp format(code) do
    try do
      Code.format_string!(code)
    rescue
      _ ->
        throw({:error, :formatting, code})
    end
  end

  @spec pretty_print_infix(infix :: String.t()) :: String.t()
  def pretty_print_infix('=:='), do: "==="
  def pretty_print_infix('=/='), do: "!=="
  def pretty_print_infix('/='), do: "!="
  def pretty_print_infix('=<'), do: "<="
  def pretty_print_infix(infix), do: to_string(infix)

  @spec pretty_print(str :: String.t()) :: String.t()
  def pretty_print(str) do
    parsed =
      str
      |> to_charlist()
      |> lex()
      |> parse()

    try do
      do_pretty_print(parsed)
    rescue
      _ ->
        throw({:error, :pretty_printing, parsed})
    end
  end

  @spec pretty_print_pattern(pattern :: String.t()) :: String.t()
  def pretty_print_pattern('pattern ' ++ rest) do
    pretty_print_type(rest)
  end

  def pretty_print_pattern(pattern) do
    pretty_print_type(pattern)
  end

  @spec pretty_print_contract(
          contract :: String.t(),
          module :: String.t(),
          function :: String.t()
        ) :: String.t()
  def pretty_print_contract(contract, module, function) do
    [head | tail] =
      contract
      |> to_string()
      |> String.split(";")

    head =
      head
      |> String.trim_leading(to_string(module))
      |> String.trim_leading(":")
      |> String.trim_leading(to_string(function))

    [head | tail]
    |> Enum.join(";")
    |> pretty_print_contract()
  end

  @spec pretty_print_contract(contract :: String.t()) :: String.t()
  def pretty_print_contract(contract) do
    [head | tail] =
      contract
      |> to_string()
      |> String.split(";")

    if Enum.empty?(tail) do
      do_pretty_print_contract(head)
    else
      joiner = "Contract head:\n"

      pretty =
        Enum.map_join([head | tail], "\n\n" <> joiner, fn contract ->
          contract
          |> to_charlist()
          |> do_pretty_print_contract()
        end)

      joiner <> pretty
    end
  end

  defp do_pretty_print_contract(contract) do
    prefix = "@spec a"
    suffix = "\ndef a() do\n  :ok\nend"
    pretty = pretty_print(contract)

    """
    @spec a#{pretty}
    def a() do
      :ok
    end
    """
    |> format()
    |> Enum.join("")
    |> String.trim_leading(prefix)
    |> String.trim_trailing(suffix)
    |> String.replace("\n      ", "\n")
  end

  @spec pretty_print_type(type :: String.t()) :: String.t()
  def pretty_print_type(type) do
    prefix = "@spec a("
    suffix = ") :: :ok\ndef a() do\n  :ok\nend"
    indented_suffix = ") ::\n        :ok\ndef a() do\n  :ok\nend"
    pretty = pretty_print(type)

    """
    @spec a(#{pretty}) :: :ok
    def a() do
      :ok
    end
    """
    |> format()
    |> Enum.join("")
    |> String.trim_leading(prefix)
    |> String.trim_trailing(suffix)
    |> String.trim_trailing(indented_suffix)
    |> String.replace("\n      ", "\n")
  end

  @spec pretty_print_args(args :: String.t()) :: String.t()
  def pretty_print_args(args) do
    prefix = "@spec a"
    suffix = " :: :ok\ndef a() do\n  :ok\nend"
    pretty = pretty_print(args)

    """
    @spec a#{pretty} :: :ok
    def a() do
      :ok
    end
    """
    |> format()
    |> Enum.join("")
    |> String.trim_leading(prefix)
    |> String.trim_trailing(suffix)
    |> String.replace("\n      ", "\n")
  end

  defp do_pretty_print({:any}) do
    "_"
  end

  defp do_pretty_print({:inner_any_function}) do
    "(...)"
  end

  defp do_pretty_print({:any_function}) do
    "(... -> any)"
  end

  defp do_pretty_print({:assignment, {:atom, atom}, value}) do
    name =
      atom
      |> deatomize()
      |> to_string()
      |> strip_var_version()

    "#{name} = #{do_pretty_print(value)}"
  end

  defp do_pretty_print({:atom, [:_]}) do
    "_"
  end

  defp do_pretty_print({:atom, ['_']}) do
    "_"
  end

  defp do_pretty_print({:atom, atom}) do
    atomize(atom)
  end

  defp do_pretty_print({:binary_part, value, _, size}) do
    "#{do_pretty_print(value)} :: #{do_pretty_print(size)}"
  end

  defp do_pretty_print({:binary_part, value, size}) do
    "#{do_pretty_print(value)} :: #{do_pretty_print(size)}"
  end

  defp do_pretty_print({:binary, [{:binary_part, {:any}, {:any}, {:size, {:int, 8}}}]}) do
    "binary()"
  end

  defp do_pretty_print({:binary, [{:binary_part, {:any}, {:any}, {:size, {:int, 1}}}]}) do
    "bitstring()"
  end

  defp do_pretty_print({:binary, binary_parts}) do
    binary_parts = Enum.map_join(binary_parts, ", ", &do_pretty_print/1)
    "<<#{binary_parts}>>"
  end

  defp do_pretty_print({:binary, value, size}) do
    "<<#{do_pretty_print(value)} :: #{do_pretty_print(size)}>>"
  end

  defp do_pretty_print({:byte_list, byte_list}) do
    byte_list
    |> Enum.into(<<>>, fn byte ->
      <<byte::8>>
    end)
    |> inspect()
  end

  defp do_pretty_print({:contract, {:args, args}, {:return, return}, {:whens, whens}}) do
    {printed_whens, when_names} = collect_and_print_whens(whens)

    args = {:when_names, when_names, args}
    return = {:when_names, when_names, return}

    "(#{do_pretty_print(args)}) :: #{do_pretty_print(return)} when #{printed_whens}"
  end

  defp do_pretty_print({:contract, {:args, {:inner_any_function}}, {:return, return}}) do
    "((...) -> #{do_pretty_print(return)})"
  end

  defp do_pretty_print({:contract, {:args, args}, {:return, return}}) do
    "#{do_pretty_print(args)} :: #{do_pretty_print(return)}"
  end

  defp do_pretty_print({:function, {:contract, {:args, args}, {:return, return}}}) do
    "(#{do_pretty_print(args)} -> #{do_pretty_print(return)})"
  end

  defp do_pretty_print({:int, int}) do
    "#{to_string(int)}"
  end

  defp do_pretty_print({:list, :paren, items}) do
    "(#{Enum.map_join(items, ", ", &do_pretty_print/1)})"
  end

  defp do_pretty_print(
         {:list, :square,
          [
            tuple: [
              {:type_list, ['a', 't', 'o', 'm'], {:list, :paren, []}},
              {:atom, [:_]}
            ]
          ]}
       ) do
    "Keyword.t()"
  end

  defp do_pretty_print(
         {:list, :square,
          [
            tuple: [
              {:type_list, ['a', 't', 'o', 'm'], {:list, :paren, []}},
              t
            ]
          ]}
       ) do
    "Keyword.t(#{do_pretty_print(t)})"
  end

  defp do_pretty_print({:list, :square, items}) do
    "[#{Enum.map_join(items, ", ", &do_pretty_print/1)}]"
  end

  defp do_pretty_print({:map_entry, key, value}) do
    "#{do_pretty_print(key)} => #{do_pretty_print(value)}"
  end

  defp do_pretty_print(
         {:map,
          [
            {:map_entry, {:atom, '\'__struct__\''}, {:atom, [:_]}},
            {:map_entry, {:atom, [:_]}, {:atom, [:_]}}
          ]}
       ) do
    "struct()"
  end

  defp do_pretty_print(
         {:map,
          [
            {:map_entry, {:atom, '\'__struct__\''},
             {:type_list, ['a', 't', 'o', 'm'], {:list, :paren, []}}},
            {:map_entry, {:type_list, ['a', 't', 'o', 'm'], {:list, :paren, []}}, {:atom, [:_]}}
          ]}
       ) do
    "struct()"
  end

  defp do_pretty_print(
         {:map,
          [
            {:map_entry, {:atom, '\'__struct__\''},
             {:type_list, ['a', 't', 'o', 'm'], {:list, :paren, []}}},
            {:map_entry, {:atom, [:_]}, {:atom, [:_]}}
          ]}
       ) do
    "struct()"
  end

  defp do_pretty_print(
         {:map,
          [
            {:map_entry, {:atom, '\'__exception__\''}, {:atom, '\'true\''}},
            {:map_entry, {:atom, '\'__struct__\''}, {:atom, [:_]}},
            {:map_entry, {:atom, [:_]}, {:atom, [:_]}}
          ]}
       ) do
    "Exception.t()"
  end

  defp do_pretty_print({:map, map_keys}) do
    %{struct_name: struct_name, entries: entries} = struct_parts(map_keys)

    if struct_name do
      "%#{struct_name}{#{Enum.map_join(entries, ", ", &do_pretty_print/1)}}"
    else
      "%{#{Enum.map_join(entries, ", ", &do_pretty_print/1)}}"
    end
  end

  defp do_pretty_print({:named_type_with_appended_colon, named_type, type})
       when is_tuple(named_type) and is_tuple(type) do
    case named_type do
      {:atom, name} ->
        name =
          name
          |> deatomize()
          |> to_string()
          |> strip_var_version()

        "#{name}: #{do_pretty_print(type)}"

      other ->
        "#{do_pretty_print(other)}: #{do_pretty_print(type)}"
    end
  end

  defp do_pretty_print({:named_type, named_type, type})
       when is_tuple(named_type) and is_tuple(type) do
    case named_type do
      {:atom, name} ->
        name =
          name
          |> deatomize()
          |> to_string()
          |> strip_var_version()

        "#{name} :: #{do_pretty_print(type)}"

      other ->
        "#{do_pretty_print(other)} :: #{do_pretty_print(type)}"
    end
  end

  defp do_pretty_print({:named_type, named_type, type}) when is_tuple(named_type) do
    case named_type do
      {:atom, name = '\'Elixir' ++ _} ->
        "#{atomize(name)}.#{deatomize(type)}()"

      {:atom, name} ->
        name =
          name
          |> deatomize()
          |> to_string()
          |> strip_var_version()

        "#{name} :: #{deatomize(type)}()"

      other ->
        name = do_pretty_print(other)
        "#{name} :: #{deatomize(type)}()"
    end
  end

  defp do_pretty_print({nil}) do
    "nil"
  end

  defp do_pretty_print({:pattern, pattern_items}) do
    "#{Enum.map_join(pattern_items, ", ", &do_pretty_print/1)}"
  end

  defp do_pretty_print(
         {:pipe_list, {:atom, ['f', 'a', 'l', 's', 'e']}, {:atom, ['t', 'r', 'u', 'e']}}
       ) do
    "boolean()"
  end

  defp do_pretty_print(
         {:pipe_list, {:atom, '\'infinity\''},
          {:type_list, ['n', 'o', 'n', :_, 'n', 'e', 'g', :_, 'i', 'n', 't', 'e', 'g', 'e', 'r'],
           {:list, :paren, []}}}
       ) do
    "timeout()"
  end

  defp do_pretty_print({:pipe_list, head, tail}) do
    "#{do_pretty_print(head)} | #{do_pretty_print(tail)}"
  end

  defp do_pretty_print({:range, from, to}) do
    "#{do_pretty_print(from)}..#{do_pretty_print(to)}"
  end

  defp do_pretty_print({:rest}) do
    "..."
  end

  defp do_pretty_print({:size, size}) do
    "size(#{do_pretty_print(size)})"
  end

  defp do_pretty_print({:tuple, tuple_items}) do
    "{#{Enum.map_join(tuple_items, ", ", &do_pretty_print/1)}}"
  end

  defp do_pretty_print({:type, type}) do
    "#{deatomize(type)}()"
  end

  defp do_pretty_print({:type, module, type}) do
    module = do_pretty_print(module)

    type =
      if is_tuple(type) do
        do_pretty_print(type)
      else
        deatomize(type) <> "()"
      end

    "#{module}.#{type}"
  end

  defp do_pretty_print({:type, module, type, inner_type}) do
    "#{atomize(module)}.#{deatomize(type)}(#{do_pretty_print(inner_type)})"
  end

  defp do_pretty_print({:type_list, type, types}) do
    "#{deatomize(type)}#{do_pretty_print(types)}"
  end

  defp do_pretty_print({:when_names, when_names, {:list, :paren, items}}) do
    Enum.map_join(items, ", ", &format_when_names(do_pretty_print(&1), when_names))
  end

  defp do_pretty_print({:when_names, when_names, item}) do
    format_when_names(do_pretty_print(item), when_names)
  end

  defp format_when_names(item, when_names) do
    trimmed = String.trim_leading(item, ":")

    if trimmed in when_names do
      downcase_first(trimmed)
    else
      item
    end
  end

  defp collect_and_print_whens(whens) do
    {pretty_names, when_names} =
      Enum.reduce(whens, {[], []}, fn {_, when_name, type}, {prettys, whens} ->
        pretty_name =
          {:named_type_with_appended_colon, when_name, type}
          |> do_pretty_print()
          |> downcase_first()

        {[pretty_name | prettys], [when_name | whens]}
      end)

    when_names =
      when_names
      |> Enum.map(fn {_, v} -> v |> atomize() |> String.trim_leading(":") end)

    printed_whens = pretty_names |> Enum.reverse() |> Enum.join(", ")

    {printed_whens, when_names}
  end

  defp downcase_first(string) do
    {first, rest} = String.split_at(string, 1)
    String.downcase(first) <> rest
  end

  defp atomize("Elixir." <> module_name) do
    String.trim(module_name, "'")
  end

  defp atomize([char]) do
    to_string(char)
  end

  defp atomize(atom) when is_list(atom) do
    atom_string =
      atom
      |> deatomize()
      |> to_string()

    stripped = strip_var_version(atom_string)

    if stripped == atom_string do
      atomize(stripped)
    else
      stripped
    end
  end

  defp atomize(<<atom>>) when is_number(atom) do
    "#{atom}"
  end

  defp atomize(atom) do
    atom = to_string(atom)

    if String.starts_with?(atom, "_") do
      atom
    else
      inspect(:"#{String.trim(atom, "'")}")
    end
  end

  defp atom_part_to_string({:int, atom_part}), do: Integer.to_charlist(atom_part)
  defp atom_part_to_string(atom_part), do: atom_part

  defp strip_var_version(var_name) do
    var_name
    |> String.replace(~r/^V(.+)@\d+$/, "\\1")
    |> String.replace(~r/^(.+)@\d+$/, "\\1")
  end

  defp struct_parts(map_keys) do
    %{struct_name: struct_name, entries: entries} =
      Enum.reduce(map_keys, %{struct_name: nil, entries: []}, &struct_part/2)

    %{struct_name: struct_name, entries: Enum.reverse(entries)}
  end

  defp struct_part({:map_entry, {:atom, '\'__struct__\''}, {:atom, struct_name}}, struct_parts) do
    struct_name =
      struct_name
      |> atomize()
      |> String.trim("\"")

    Map.put(struct_parts, :struct_name, struct_name)
  end

  defp struct_part(entry, struct_parts = %{entries: entries}) do
    Map.put(struct_parts, :entries, [entry | entries])
  end

  defp deatomize([:_, :_, '@', {:int, _}]) do
    "_"
  end

  defp deatomize(chars) when is_list(chars) do
    Enum.map(chars, fn char ->
      char
      |> deatomize_char()
      |> atom_part_to_string()
    end)
  end

  defp deatomize_char(char) when is_atom(char) do
    Atom.to_string(char)
  end

  defp deatomize_char(char), do: char
end
