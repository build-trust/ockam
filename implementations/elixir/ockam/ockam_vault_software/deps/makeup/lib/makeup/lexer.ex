defmodule Makeup.Lexer do
  @moduledoc """
  A lexer turns raw source code into a list of tokens.
  """
  alias Makeup.Lexer.Types, as: T
  alias Makeup.Lexer.Postprocess

  @doc """
  Parses the smallest number of tokens that make sense.
  It's a `parsec`.
  """
  @callback root_element(String.t) :: T.parsec_result

  @doc """
  Parses the given string into a `parsec` result that inludes a list of tokens.
  """
  @callback root(String.t) :: T.parsec_result

  @doc """
  Postprocesses a list of tokens before matching the contained groups.
  """
  @callback postprocess([T.token()], list()) :: [T.token()]

  @doc """
  Matches groups in a list of tokens.
  """
  @callback match_groups([T.token()], String.t) :: [T.token()]

  @doc """
  Lexes a string into a list of tokens
  """
  @callback lex(String.t(), list()) :: [T.token()]


  @doc """
  Merges the token values into the original string.

  Inverts the ouput of a lexer. That is, if `lexer` is a lexer, then:

      string |> lexer.lex() |> Makeup.Lexer.unlex() == string

  This only works for a correctly implemented lexer, of course.
  The above identity can be trated as a lexer invariant for newly implemented lexers.
  """
  @spec unlex(list(T.token())) :: String.t()
  def unlex(tokens) do
    tokens
    |> Enum.map(&Postprocess.token_value_to_binary/1)
    |> Enum.map(fn {_tag, _meta, value} -> value end)
    |> Enum.join()
  end

  @doc """
  Splits a list of tokens on newline characters (`\n`).

  The result is a list of lists of tokens with no newlines.
  """
  @spec split_into_lines(list(T.token())) :: list(list(T.token()))
  def split_into_lines(tokens) do
    {lines, last_line} =
      Enum.reduce tokens, {[], []}, (fn token, {lines, line} ->
        {ttype, meta, text} = Postprocess.token_value_to_binary(token)
        case String.split(text, "\n") do
          [_] -> {lines, [token | line]}
          [part | parts] ->
            first_line = [{ttype, meta, part} | line] |> :lists.reverse

            all_but_last_line =
              parts
              |> Enum.slice(0..-2)
              |> Enum.map(fn tok_text -> [{ttype, meta, tok_text}] end)
              |> :lists.reverse

            last_line = [{ttype, meta, Enum.at(parts, -1)}]

            {all_but_last_line ++ [first_line | lines], last_line}
        end
      end)

    :lists.reverse([last_line | lines])
  end

  @doc """
  Merge adjacent tokens of the same type and with the same attributes.

  Doing this will require iterating over the list of tokens again,
  so only do this if you have a good reason.
  """
  @spec merge(list(T.token())) :: list(T.token())
  def merge([{tag, meta, value1}, {tag, meta, value2} | rest]),
    do: merge [{tag, meta, value1 <> value2} | rest]
  def merge([token | rest]),
    do: [token | merge(rest)]
  def merge([]),
    do: []
end
