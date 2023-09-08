defmodule Makeup.Lexer.Postprocess do
  @moduledoc """
  Often you'll want to run the token list through a postprocessing stage before
  running the formatter.

  Most of what we can do in a post-processing stage can be done with more parsing rules,
  but doing it in a post-processing stage is often easier and faster.
  Never assume one of the options is faster than the other, always measure performance.
  """

  @doc """
  Takes a list of the format `[{key1, [val11, val12, ...]}, {key2, [val22, ...]}]` and
  returns a map of the form `%{val11 => key1, val12 => key2, ..., val22 => key2, ...}`.

  The resulting map may be useful to highlight some tokens in a special way
  in a postprocessing step.

  You can also use pattern matching instead of the inverted map,
  and it will probably be faster, but always benchmark the alternatives before
  commiting to an implementation.
  """
  def invert_word_map(pairs) do
    nested =
      for {ttype, words} <- pairs do
        for word <- words, do: {word, ttype}
      end

    nested
    |> List.flatten()
    |> Enum.into(%{})
  end

  @doc """
  Converts the value of a token into a binary.

  Token values are usually iolists for performance reasons.
  The BEAM is actually quite fast at printing or concatenating iolists,
  and some of the basic combinators output iolists, so there is no need
  to convert the token values into binaries.

  This function should only be used for tesring purposes, when you might
  want to compare the token list into a reference output.

  Converting the tokens into binaries has two advantges:
  1. It's much easier to compare tokens by visual inspection when the value is a binary
  2. When testing, two iolists that print to the same binary should be considered equal.

  This function hasn't been optimized for speed.
  Don't use in production code.
  """
  def token_value_to_binary({ttype, meta, value}) do
    {ttype, meta, to_string([value])}
  end

  @doc """
  Converts the values of the tokens in the list into binaries.

  Token values are usually iolists for performance reasons.
  The BEAM is actually quite fast at printing or concatenating iolists,
  and some of the basic combinators output iolists, so there is no need
  to convert the token values into binaries.

  This function should only be used for tesring purposes, when you might
  want to compare the token list into a reference output.

  Converting the tokens into binaries has two advantges:
  1. It's much easier to compare tokens by visual inspection when the value is a binary
  2. When testing, two iolists that print to the same binary should be considered equal.

  ## Example

  ```elixir
  defmodule MyTest do
    use ExUnit.Case
    alias Makeup.Lexers.ElixirLexer
    alias Makeup.Lexer.Postprocess

    test "binaries are much easier on the eyes" do
      naive_tokens = ElixirLexer(":atom")
      # Hard to inspect visually
      assert naive_tokens == [{:string_symbol, %{language: :elixir}, [":", "a", "tom"]}]
      better_tokens =
        text
        |> ElixirLexer.lex()
        |> Postprocess.token_values_to_binaries()
      # Easy to inspect visually
      assert better_tokens == [{:string_symbol, %{language: :elixir}, ":atom"}]
    end
  end
  ```

  Actually, you'll want to define some kind of helper to make it less verbose.
  For example:

  ```elixir
  defmodule MyTest do
    use ExUnit.Case
    alias Makeup.Lexers.ElixirLexer
    alias Makeup.Lexer.Postprocess

    def lex(text) do
      text
      |> ElixirLexer.lex(group_prefix: "group")
      |> Postprocess.token_values_to_binaries()
    end

    test "even better with our little helper" do
      assert lex(":atom") == [{:string_symbol, %{language: :elixir}, ":atom"}]
    end
  end
  """
  def token_values_to_binaries(tokens) do
    Enum.map(tokens, &token_value_to_binary/1)
  end
end
