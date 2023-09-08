defmodule Makeup.Lexers.ElixirLexer.Helper do
  @moduledoc false
  import NimbleParsec
  alias Makeup.Lexer.Combinators

  def with_optional_separator(combinator, separator) when is_binary(separator) do
    combinator |> repeat(string(separator) |> concat(combinator))
  end

  # Allows escaping of the first character of a right delimiter.
  # This is used in sigils that don't support interpolation or character escapes but
  # must support escaping of the right delimiter.
  def escape_delim(rdelim) do
    rdelim_first_char = String.slice(rdelim, 0..0)
    string("\\" <> rdelim_first_char)
  end

  def sigil(ldelim, rdelim, ranges, middle) do
    left = string("~") |> utf8_char(ranges) |> string(ldelim)
    right = string(rdelim)

    choices = middle ++ [utf8_char([])]

    left
    |> repeat(lookahead_not(right) |> choice(choices))
    |> concat(right)
    |> optional(utf8_string([?a..?z, ?A..?Z, ?0..?9], min: 1))
    |> post_traverse({__MODULE__, :build_sigil, []})
  end

  def build_sigil(rest, acc, context, line, offset) do
    type =
      case Enum.at(acc, -2) do
        sigil when sigil in 'sScC' -> :string
        sigil when sigil in 'rR' -> :string_regex
        sigil when sigil in 'TDNU' -> :literal_date
        _ -> :string_sigil
      end

    Combinators.collect_raw_chars_and_binaries(rest, acc, context, line, offset, type, %{})
  end

  def escaped(literal) when is_binary(literal) do
    string("\\" <> literal)
  end

  def keyword_matcher(kind, fun_name, words) do
    heads =
      for {ttype, words} <- words do
        for word <- words do
          case kind do
            :defp ->
              quote do
                defp unquote(fun_name)([{:name, attrs, unquote(ttype)} | tokens]) do
                  [{unquote(ttype), attrs, unquote(word)} | unquote(fun_name)(tokens)]
                end
              end
              |> IO.inspect()

            :def ->
              quote do
                def unquote(fun_name)([{:name, attrs, unquote(ttype)} | tokens]) do
                  [{unquote(ttype), attrs, unquote(word)} | unquote(fun_name)(tokens)]
                end
              end
          end
        end
      end

    quote do
      (unquote_splicing(heads))
    end
  end
end
