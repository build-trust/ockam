defmodule ExDoc.ShellLexer do
  # Makeup lexer for sh, bash, etc commands.
  # The only thing it does is making the `$ ` prompt not selectable.
  @moduledoc false

  @behaviour Makeup.Lexer

  @impl true
  def lex(text, _opts) do
    text
    |> String.split("\n")
    |> Enum.flat_map(fn
      "$ " <> rest ->
        [
          {:generic_prompt, %{selectable: false}, "$ "},
          {:text, %{}, rest <> "\n"}
        ]

      text ->
        [{:text, %{}, text <> "\n"}]
    end)
  end

  @impl true
  def match_groups(_arg0, _arg1) do
    raise "not implemented yet"
  end

  @impl true
  def postprocess(_arg0, _arg1) do
    raise "not implemented yet"
  end

  @impl true
  def root(_arg0) do
    raise "not implemented yet"
  end

  @impl true
  def root_element(_arg0) do
    raise "not implemented yet"
  end
end
