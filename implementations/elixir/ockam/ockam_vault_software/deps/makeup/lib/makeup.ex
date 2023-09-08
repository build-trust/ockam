defmodule Makeup do
  @moduledoc """
  Syntax highlighting library for code, inspired by Pygments.

  By default, it doesn't include any lexers. You must import
  them separately (even the Elixir lexer).
  """
  alias Makeup.Formatters.HTML.HTMLFormatter
  alias Makeup.Lexers.ElixirLexer
  alias Makeup.Styles.HTML.StyleMap
  alias Makeup.Styles.HTML.Style
  alias Makeup.Registry
  require StyleMap

  @doc """
  Highlights the given string using the given lexer and formatter.

  By default it highlight the Elixir language using HTML
  """
  def highlight(source, options \\ []) do
    {lexer, lexer_options} = fetch_lexer(options)

    formatter =
      case options[:formatter] do
        nil -> HTMLFormatter
        module when is_atom(module) -> module
      end

    tokens = apply(lexer, :lex, [source, lexer_options])
    apply(formatter, :format_as_binary, [tokens])
  end

  def highlight_inner_html(source, options \\ []) do
    {lexer, lexer_options} = fetch_lexer(options)
    formatter_options = Keyword.get(options, :formatter_options, [])

    tokens = apply(lexer, :lex, [source, lexer_options])
    apply(HTMLFormatter, :format_inner_as_binary, [tokens, formatter_options])
  end

  defp fetch_lexer(options) do
    {lexer, lexer_options} =
      case options[:lexer] do
        nil -> {ElixirLexer, []}
        module when is_atom(module) -> {module, []}
        name -> Registry.fetch_lexer_by_name!(name)
      end

    {lexer, Keyword.merge(lexer_options, Keyword.get(options, :lexer_options, []))}
  end

  @doc """
  Generates a CSS stylesheet for highlighted code for the given style.

  It expects a `style`, either as an atom name or as `StyleMap`, and the
  `css_class` as the top level class for highlighted code.

  Ff the `css_class` is `"highlight"` (the default), the stylesheet has
  the form:

  ```css
  .highlight .someclass {...}
  .highlight .anotherclass {...}
  ```

  See `Makeup.Styles.HTML.StyleMap` for all style maps.
  """
  def stylesheet(style \\ StyleMap.default_style(), css_class \\ "highlight")

  def stylesheet(style, css_class) when is_atom(style) do
    stylesheet(apply(StyleMap, style, []), css_class)
  end

  def stylesheet(style, css_class) do
    Style.stylesheet(style, css_class)
  end
end
